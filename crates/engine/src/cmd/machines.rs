//! Building blocks for interactive commands: object selection and simple point sequences.

use cadcraft_doc::Handle;
use serde_json::{Value, json};

use crate::{Accept, EngineError, Input, Interactive, Prompt, Result, Session, Step};

/// A "Select objects:" phase. Uses the pickfirst selection when there is one.
#[derive(Clone, Debug, Default)]
pub struct SelectPhase {
    pub picked: Vec<Handle>,
    pub done: bool,
    pub single: bool,
    pub removing: bool,
    /// What each selection so far changed, for Undo: (handles, whether they were removed).
    pub history: Vec<(Vec<Handle>, bool)>,
}

/// Most selection steps Undo remembers.
const MAX_SELECT_HISTORY: usize = 1000;

pub enum SelOutcome {
    /// Keep prompting.
    More,
    /// Selection finished with these objects.
    Done(Vec<Handle>),
    /// Selection finished empty: the command should end.
    Empty,
}

impl SelectPhase {
    /// Start with the pickfirst selection, if any (then the phase is already done).
    pub fn begin(s: &mut Session) -> Self {
        let pre = s.selection();
        if !pre.is_empty() && s.settings.pickfirst {
            s.remember_selection(&pre);
            SelectPhase { picked: pre, done: true, ..Default::default() }
        } else {
            SelectPhase::default()
        }
    }
    pub fn single() -> Self {
        SelectPhase { single: true, ..Default::default() }
    }
    pub fn prompt(&self) -> Prompt {
        let msg = if self.removing {
            "Remove objects"
        } else if self.single {
            "Select object"
        } else {
            "Select objects"
        };
        Prompt::new(msg, Accept::SELECT)
    }
    pub fn feed(&mut self, s: &mut Session, i: &Input) -> Result<SelOutcome> {
        match i {
            Input::Pick(hs) => {
                let changed: Vec<Handle> = if self.removing {
                    let gone: Vec<Handle> = self.picked.iter().copied().filter(|h| hs.contains(h)).collect();
                    self.picked.retain(|h| !hs.contains(h));
                    gone
                } else {
                    let mut added = Vec::new();
                    for h in hs {
                        if !self.picked.contains(h) {
                            self.picked.push(*h);
                            added.push(*h);
                        }
                    }
                    added
                };
                if !changed.is_empty() {
                    if self.history.len() >= MAX_SELECT_HISTORY {
                        self.history.remove(0);
                    }
                    self.history.push((changed, self.removing));
                }
                let n = hs.len();
                if n > 0 {
                    s.echo(format!("{n} found, {} total", self.picked.len()));
                } else {
                    s.echo("0 found");
                }
                s.set_selection(self.picked.clone());
                if self.single && !self.picked.is_empty() {
                    return Ok(self.finish(s));
                }
                Ok(SelOutcome::More)
            }
            Input::Text(t) | Input::Keyword(t) => {
                match t.trim().to_ascii_lowercase().as_str() {
                    "r" | "remove" => self.removing = true,
                    "a" | "add" => self.removing = false,
                    // Undo: take back the last selection that changed the set.
                    "u" | "undo" => {
                        match self.history.pop() {
                            Some((hs, true)) => self.picked.extend(hs),
                            Some((hs, false)) => self.picked.retain(|h| !hs.contains(h)),
                            None => s.echo("Nothing to undo."),
                        }
                        s.echo(format!("{} total", self.picked.len()));
                        s.set_selection(self.picked.clone());
                    }
                    _ => s.echo("*Invalid selection*"),
                }
                Ok(SelOutcome::More)
            }
            Input::Enter => Ok(self.finish(s)),
            _ => Ok(SelOutcome::More),
        }
    }
    fn finish(&mut self, s: &mut Session) -> SelOutcome {
        self.done = true;
        if self.picked.is_empty() {
            SelOutcome::Empty
        } else {
            s.remember_selection(&self.picked);
            SelOutcome::Done(self.picked.clone())
        }
    }
}

/// "Select objects:", then run a JSON command on them as `{handles}`. With a pickfirst selection it
/// runs at once, exactly like the JSON form does on the current selection. A repeating run acts on
/// every pick and asks for the next object, the way AutoCAD's LAYOFF does.
pub struct SelectRun {
    id: &'static str,
    name: &'static str,
    /// Single-object prompt (e.g. LAYFRZ's "Select an object on the layer to be frozen").
    single: Option<&'static str>,
    /// Stay active after each pick, until Enter or Esc (LAYOFF).
    repeat: bool,
    sel: SelectPhase,
}

impl SelectRun {
    pub fn new(id: &'static str, name: &'static str) -> Self {
        SelectRun { id, name, single: None, repeat: false, sel: SelectPhase::default() }
    }
    /// Pick one object, with a command-specific prompt.
    pub fn single(id: &'static str, name: &'static str, msg: &'static str) -> Self {
        SelectRun { single: Some(msg), ..SelectRun::new(id, name) }
    }
    /// Get an object, act on it and ask again, until Enter or Esc: AutoCAD's Layer Off (LAYOFF)
    /// turns off the layer of every object picked, one after another, in one command.
    pub fn repeat(id: &'static str, name: &'static str, msg: &'static str) -> Self {
        SelectRun { single: Some(msg), repeat: true, ..SelectRun::new(id, name) }
    }
    fn run(&self, s: &mut Session, hs: &[Handle]) -> Result<()> {
        let spec = super::find_command(self.id).ok_or_else(|| EngineError::UnknownCommand(self.id.into()))?;
        // Call the command body directly: the running command records the single undo step.
        let r = (spec.run)(s, &json!({ "handles": hs.iter().map(|h| h.hex()).collect::<Vec<_>>() }))?;
        if let Some(m) = r.get("message").and_then(Value::as_str) {
            s.echo(m.to_string());
        }
        Ok(())
    }
    /// One pick of a repeating run: act on it at once and keep the prompt up.
    fn pick(&self, s: &mut Session, hs: &[Handle]) -> Result<Step> {
        if hs.is_empty() {
            s.echo("0 found");
            return Ok(Step::Continue);
        }
        self.run(s, hs)?;
        // Objects picked at the prompt don't stay selected afterwards.
        s.set_selection(Vec::new());
        Ok(Step::Continue)
    }
}

impl Interactive for SelectRun {
    fn name(&self) -> &'static str {
        self.name
    }
    fn begin(&mut self, s: &mut Session) -> Result<Step> {
        self.sel = SelectPhase::begin(s);
        if self.sel.done {
            let hs = self.sel.picked.clone();
            self.run(s, &hs)?;
            return Ok(Step::Done);
        }
        // A repeating run acts on every pick, so no single pick may finish it.
        self.sel.single = self.single.is_some() && !self.repeat;
        Ok(Step::Continue)
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        match self.single {
            // A repeating run keeps asking for the next object, whatever selection keywords were typed.
            Some(msg) if self.repeat || !self.sel.removing => Prompt::new(msg, Accept::SELECT),
            _ => self.sel.prompt(),
        }
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        // A repeating run acts on each pick and prompts again; Enter (or Esc) ends it.
        if self.repeat {
            return match i {
                Input::Pick(hs) => self.pick(s, &hs),
                Input::Enter => Ok(Step::Done),
                other => self.sel.feed(s, &other).map(|_| Step::Continue),
            };
        }
        match self.sel.feed(s, &i)? {
            SelOutcome::More => Ok(Step::Continue),
            SelOutcome::Empty => Ok(Step::Done),
            SelOutcome::Done(hs) => {
                // Objects picked at the prompt don't stay selected afterwards.
                self.run(s, &hs)?;
                s.set_selection(Vec::new());
                Ok(Step::Done)
            }
        }
    }
}

/// Parse a number from text input.
pub fn number(t: &str) -> Option<f64> {
    crate::units::parse_distance(t)
}

/// The parameters a command's params doc (`{name, color?, …}`) marks as required: the top-level
/// names without a `?`. Empty when the doc isn't a plain list of names.
pub fn required_params(doc: &str) -> Vec<String> {
    let Some(body) = doc.trim_start().strip_prefix('{') else { return Vec::new() };
    let mut depth = 0i32;
    let mut end = None;
    for (i, c) in body.char_indices() {
        match c {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' if depth == 0 => {
                end = Some(i);
                break;
            }
            '}' | ']' | ')' => depth -= 1,
            _ => {}
        }
    }
    let Some(end) = end else { return Vec::new() };
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    let inner = &body[..end];
    let mut fields = Vec::new();
    for (i, c) in inner.char_indices() {
        match c {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                fields.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    fields.push(&inner[start..]);
    for f in fields {
        let f = f.trim_start();
        let name: String = f.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && !f[name.len()..].starts_with('?') {
            out.push(name);
        }
    }
    out
}

/// A command that only has a JSON form, typed at the command line or in a script without its
/// required parameters: ask for each in turn ("Enter name"), then run it with them. Each value is
/// passed as typed text first (a layer named `0` stays a name); if the command refuses that, a
/// value that reads as a JSON number, boolean, array or object is passed as one instead. A refusal
/// ends the command, so the lines after it in a script are not read as its parameters.
pub struct ParamsRun {
    id: &'static str,
    keys: Vec<String>,
    got: serde_json::Map<String, Value>,
}

impl ParamsRun {
    pub fn new(id: &'static str, keys: Vec<String>) -> Self {
        ParamsRun { id, keys, got: serde_json::Map::new() }
    }
}

/// `params` with each text value that reads as another JSON value (a number, a list) as that value;
/// `None` when there is none.
fn typed_values(params: &serde_json::Map<String, Value>) -> Option<Value> {
    let mut changed = false;
    let out: serde_json::Map<String, Value> = params
        .iter()
        .map(|(k, v)| {
            let parsed = v.as_str().and_then(|t| serde_json::from_str::<Value>(t).ok()).filter(|p| !p.is_string() && !p.is_null());
            changed |= parsed.is_some();
            (k.clone(), parsed.unwrap_or_else(|| v.clone()))
        })
        .collect();
    changed.then_some(Value::Object(out))
}

impl Interactive for ParamsRun {
    fn name(&self) -> &'static str {
        self.id
    }
    fn prompt(&self, _s: &Session) -> Prompt {
        let key = self.keys.get(self.got.len()).map_or("value", String::as_str);
        Prompt::new(format!("Enter {key}"), Accept::TEXT)
    }
    fn input(&mut self, s: &mut Session, i: Input) -> Result<Step> {
        let text = match i {
            Input::Text(t) | Input::Keyword(t) => t,
            Input::Cancel | Input::Enter => return Ok(Step::Cancel),
            _ => return Ok(Step::Continue),
        };
        let Some(key) = self.keys.get(self.got.len()).cloned() else { return Ok(Step::Done) };
        self.got.insert(key, Value::String(text.trim().to_string()));
        if self.got.len() < self.keys.len() {
            return Ok(Step::Continue);
        }
        let spec = super::find_command(self.id).ok_or_else(|| EngineError::UnknownCommand(self.id.into()))?;
        let got = std::mem::take(&mut self.got);
        // Call the command body directly: the running command records the single undo step.
        let r = match (spec.run)(s, &Value::Object(got.clone())) {
            Err(EngineError::BadParams { .. }) if let Some(typed) = typed_values(&got) => (spec.run)(s, &typed),
            r => r,
        };
        match r {
            Ok(r) => {
                if let Some(m) = r.as_str().or_else(|| r.get("message").and_then(Value::as_str)) {
                    s.echo(m.to_string());
                }
                Ok(Step::Done)
            }
            Err(e) => {
                s.echo(e.to_string());
                Ok(Step::Cancel)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cadcraft_doc::{EntityKind, Handle};
    use cadcraft_geom::Vec2;
    use serde_json::json;

    use crate::{Input, Session};

    /// Menu/typed invocation of selection commands prompts "Select objects" when nothing is selected
    /// (#126), and acts on the picked objects in one undo step.
    #[test]
    fn selection_commands_prompt_without_pickfirst() {
        let mut s = Session::new();
        s.execute("layer.new", &json!({ "name": "A", "current": true })).unwrap();
        s.execute("line", &json!({ "points": [[0, 0], [10, 0]] })).unwrap();
        s.execute("layer.current", &json!({ "name": "0" })).unwrap();
        s.execute("line", &json!({ "points": [[0, 5], [10, 5]] })).unwrap();
        let hs: Vec<Handle> = s.doc().unwrap().model.iter().map(|e| e.handle).collect();
        let undo_before = s.state().unwrap().undo.len();

        s.start("cutclip").unwrap();
        assert!(s.prompt_text().contains("Select objects"), "{}", s.prompt_text());
        s.input(Input::Pick(vec![hs[1]])).unwrap();
        s.input(Input::Enter).unwrap();
        assert!(s.running.is_none());
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert_eq!(s.clipboard.len(), 1);
        assert_eq!(s.state().unwrap().undo.len(), undo_before + 1, "one undo step");

        // COPYBASE asks for the base point first, then for objects.
        s.start("copybase").unwrap();
        assert!(s.prompt_text().contains("Specify base point"), "{}", s.prompt_text());
        s.input(Input::Point(Vec2::new(3.0, 4.0))).unwrap();
        assert!(s.prompt_text().contains("Select objects"), "{}", s.prompt_text());
        s.input(Input::Pick(vec![hs[0]])).unwrap();
        s.input(Input::Enter).unwrap();
        assert!(s.running.is_none());
        assert_eq!(s.clipboard_base, Vec2::new(3.0, 4.0));

        // LAYFRZ picks one object and freezes its layer.
        s.start("layfrz").unwrap();
        assert!(s.prompt_text().contains("layer to be frozen"), "{}", s.prompt_text());
        s.input(Input::Pick(vec![hs[0]])).unwrap();
        assert!(s.running.is_none());
        assert!(s.doc().unwrap().layer("A").is_some_and(|l| l.frozen));
    }

    /// With a pickfirst selection the commands run at once; JSON calls never prompt.
    #[test]
    fn selection_commands_pickfirst_and_json_unchanged() {
        let mut s = Session::new();
        s.execute("line", &json!({ "points": [[0, 0], [10, 0]] })).unwrap();
        let h = s.doc().unwrap().model.iter().next().unwrap().handle;
        s.set_selection(vec![h]);
        s.start("copyclip").unwrap();
        assert!(s.running.is_none(), "pickfirst: no prompt");
        assert_eq!(s.clipboard.len(), 1);

        s.set_selection(Vec::new());
        for id in ["cutclip", "copyclip", "zoom.object", "draworder.front", "laycur", "layoff"] {
            let _ = s.execute(id, &json!({}));
            assert!(s.running.is_none(), "{id}: JSON form must not prompt");
        }
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert!(s.doc().unwrap().layer("0").is_some_and(|l| l.on));
    }

    /// The cursor shows the pick box exactly where a click picks objects (#34): "Select objects",
    /// single-object prompts, and the idle crosshair when PICKFIRST is on.
    #[test]
    fn pick_box_at_object_selection_prompts() {
        let mut s = Session::new();
        s.execute("line", &json!({"points": [[0, 0], [10, 0]]})).unwrap();
        assert!(s.picking_objects(), "idle with PICKFIRST on");
        s.settings.pickfirst = false;
        assert!(!s.picking_objects(), "idle with PICKFIRST off");
        s.settings.pickfirst = true;
        for (cmd, picks) in [("erase", true), ("fillet", true), ("offset", false), ("line", false), ("hatch", false)] {
            s.cmdline(cmd).unwrap();
            assert_eq!(s.picking_objects(), picks, "{cmd}: {}", s.prompt_text());
            s.cancel();
        }
        // OFFSET's object pick, FILLET's radius, HATCH's select mode.
        s.cmdline("offset 2").unwrap();
        assert!(s.picking_objects(), "{}", s.prompt_text());
        s.cancel();
        s.cmdline("fillet r").unwrap();
        assert!(!s.picking_objects(), "{}", s.prompt_text());
        s.cancel();
        s.cmdline("hatch s").unwrap();
        assert!(s.picking_objects(), "{}", s.prompt_text());
        s.cancel();
    }

    /// Commands offer the option and values used last time as the `<default>`, and Enter takes it
    /// (#34). JSON calls keep their documented defaults.
    #[test]
    fn prompts_remember_last_options() {
        let mut s = Session::new();
        s.execute("rectang", &json!({"p1": [0, 0], "p2": [10, 10]})).unwrap();
        let rect = s.doc().unwrap().model.iter().next().unwrap().handle;

        // HATCH: the "Select objects" mode sticks; `P` there is still Previous, not "picK".
        s.cmdline("hatch s").unwrap();
        assert!(s.prompt_text().contains("Select objects or [picK internal point"), "{}", s.prompt_text());
        s.input(Input::Pick(vec![rect])).unwrap();
        s.cmdline("").unwrap();
        assert!(s.running.is_none(), "Enter creates the hatch");
        let kinds: Vec<&str> = s.doc().unwrap().model.iter().map(|e| e.kind.type_name()).collect();
        assert_eq!(kinds, vec!["Hatch", "Polyline"]);
        s.cmdline("hatch").unwrap();
        assert!(s.prompt_text().contains("HATCH Select objects"), "{}", s.prompt_text());
        s.cmdline("p").unwrap();
        assert!(s.prompt_text().contains("HATCH Select objects"), "{}", s.prompt_text());
        // Pattern scale: the current value is the default, Enter keeps it.
        s.cmdline("c").unwrap();
        assert!(s.prompt_text().ends_with("<1.0000>:"), "{}", s.prompt_text());
        s.cmdline("2").unwrap();
        s.cmdline("c").unwrap();
        assert!(s.prompt_text().ends_with("<2.0000>:"), "{}", s.prompt_text());
        s.cmdline("").unwrap();
        s.cmdline("k").unwrap();
        s.cancel();
        s.cmdline("hatch").unwrap();
        assert!(s.prompt_text().contains("HATCH Pick internal point"), "{}", s.prompt_text());
        s.cancel();

        // POLYGON: sides and Inscribed/Circumscribed.
        s.cmdline("polygon 5 30,0 c 2").unwrap();
        s.cmdline("polygon").unwrap();
        assert!(s.prompt_text().ends_with("<5>:"), "{}", s.prompt_text());
        s.cmdline("").unwrap();
        s.cmdline("40,0").unwrap();
        assert!(s.prompt_text().ends_with("<C>:"), "{}", s.prompt_text());
        s.cancel();

        // FILLET radius and CHAMFER distances (the second defaults to the first).
        s.cmdline("fillet r 2").unwrap();
        s.cmdline("r").unwrap();
        assert!(s.prompt_text().ends_with("<2.0000>:"), "{}", s.prompt_text());
        s.cancel();
        s.cmdline("chamfer d 1.5").unwrap();
        assert!(s.prompt_text().contains("second chamfer distance <1.5000>:"), "{}", s.prompt_text());
        s.cmdline("").unwrap();
        assert_eq!(s.doc().unwrap().header.f64("CHAMFERB", 0.0), 1.5);
        s.cancel();

        // ROTATE angle and SCALE factor.
        s.set_selection(vec![rect]);
        s.cmdline("rotate 0,0 90").unwrap();
        s.set_selection(vec![rect]);
        s.cmdline("rotate 0,0").unwrap();
        assert!(s.prompt_text().ends_with("<90>:"), "{}", s.prompt_text());
        s.cmdline("").unwrap();
        assert!(s.running.is_none());
        s.set_selection(vec![rect]);
        s.cmdline("scale 0,0 2").unwrap();
        s.set_selection(vec![rect]);
        s.cmdline("scale 0,0").unwrap();
        assert!(s.prompt_text().ends_with("<2.0000>:"), "{}", s.prompt_text());
        s.cancel();

        // JSON calls are not affected by what was remembered.
        let r = s.execute("polygon", &json!({"center": [60, 0], "radius": 1, "sides": 4})).unwrap();
        let h = Handle::parse_hex(r["handle"].as_str().unwrap()).unwrap();
        match &s.doc().unwrap().entity(h).unwrap().kind {
            EntityKind::LwPolyline(p) => assert!((p.vertices[0].p.dist(Vec2::new(60.0, 0.0)) - 1.0).abs() < 1e-9, "inscribed"),
            other => panic!("{other:?}"),
        }
    }
}
