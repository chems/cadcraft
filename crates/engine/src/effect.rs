//! What a command changed, for automation clients: a command that replies nothing (`null`) leaves
//! an agent unable to tell whether it ran. [`Session::execute_reported`] and the app's control
//! channel reply this summary instead: objects created (their handles), modified and erased,
//! layers added or changed, the current layer and the view.

use std::collections::HashMap;

use cadcraft_doc::{Drawing, Entity, Handle};
use serde_json::{Value, json};

use crate::{Arc, Session};

/// At most this many handles are listed; `createdCount` gives the total.
const LISTED: usize = 50;

/// The drawing a command starts from: which open drawing it is, and its state.
#[derive(Clone)]
pub struct Before {
    uid: u64,
    doc: Arc<Drawing>,
}

/// The active drawing before a command, to compare with afterwards.
pub fn before(s: &Session) -> Option<Before> {
    s.state().ok().map(|st| Before { uid: st.uid, doc: st.doc.clone() })
}

/// `v` when the command replied something; otherwise what it changed since `before`.
pub fn or_summary(v: Value, id: &str, before: Option<&Before>, s: &Session) -> Value {
    if !v.is_null() {
        return v;
    }
    let Ok(st) = s.state() else { return json!({"ok": true, "command": id}) };
    let view = st.view();
    let mut out = json!({
        "ok": true,
        "command": id,
        "currentLayer": st.doc.header.str("CLAYER", "0"),
        "view": {"center": [view.center.x, view.center.y], "height": view.height},
    });
    match before {
        Some(b) if b.uid == st.uid => {
            if let Some(o) = out.as_object_mut() {
                for (k, v) in changes(&b.doc, &st.doc) {
                    o.insert(k.into(), v);
                }
            }
        }
        // Another drawing is active now (a command that opens, creates or switches drawings).
        _ => out["drawing"] = json!(st.title),
    }
    out
}

fn entities(d: &Drawing) -> impl Iterator<Item = &Arc<Entity>> {
    d.model.iter().chain(d.layouts.iter().flat_map(|l| l.entities.iter()))
}

/// The differences between two states of one drawing.
fn changes(a: &Drawing, b: &Drawing) -> Vec<(&'static str, Value)> {
    if std::ptr::eq(a, b) {
        return vec![("changed", json!(false))];
    }
    let old: HashMap<Handle, &Arc<Entity>> = entities(a).map(|e| (e.handle, e)).collect();
    let (mut created, mut modified, mut kept) = (Vec::new(), 0usize, 0usize);
    for e in entities(b) {
        match old.get(&e.handle) {
            None => created.push(e.handle),
            Some(o) => {
                kept += 1;
                if !Arc::ptr_eq(o, e) && o.as_ref() != e.as_ref() {
                    modified += 1;
                }
            }
        }
    }
    let erased = old.len().saturating_sub(kept);
    let layers_added: Vec<&str> = b.layers.iter().filter(|l| a.layer(&l.name).is_none()).map(|l| l.name.as_str()).collect();
    let layers_changed: Vec<&str> =
        b.layers.iter().filter(|l| a.layer(&l.name).is_some_and(|o| o != *l)).map(|l| l.name.as_str()).collect();
    let mut out = vec![
        ("changed", json!(!created.is_empty() || modified > 0 || erased > 0 || a != b)),
        ("createdCount", json!(created.len())),
        ("created", json!(created.iter().take(LISTED).map(|h| h.hex()).collect::<Vec<_>>())),
        ("modified", json!(modified)),
        ("erased", json!(erased)),
    ];
    if !layers_added.is_empty() {
        out.push(("layersAdded", json!(layers_added)));
    }
    if !layers_changed.is_empty() {
        out.push(("layersChanged", json!(layers_changed)));
    }
    out
}

impl Session {
    /// [`Session::execute`], replying what the command changed when it replies nothing itself.
    pub fn execute_reported(&mut self, id: &str, params: &Value) -> crate::Result<Value> {
        let b = before(self);
        let v = self.execute(id, params)?;
        Ok(or_summary(v, id, b.as_ref(), self))
    }
}
