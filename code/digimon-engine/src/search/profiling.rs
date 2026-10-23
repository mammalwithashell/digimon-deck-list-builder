//! Opt-in, thread-local wall-clock spans. No RNG or search state is touched.
//! `self_seconds` excludes nested spans; inclusive totals must not be summed.
use std::{cell::RefCell, collections::BTreeMap, marker::PhantomData, rc::Rc, time::Instant};

#[derive(Default)]
struct Profile {
    enabled: bool,
    stack: Vec<(&'static str, Instant, f64)>,
    entries: BTreeMap<&'static str, (u64, f64, f64)>,
}
thread_local! { static PROFILE: RefCell<Profile> = RefCell::new(Profile::default()); }

pub fn reset(enabled: bool) {
    PROFILE.with(|p| {
        *p.borrow_mut() = Profile {
            enabled,
            ..Profile::default()
        }
    });
}

pub struct Span(bool, PhantomData<Rc<()>>);
pub fn span(name: &'static str) -> Span {
    let enabled = PROFILE.with(|p| {
        let mut p = p.borrow_mut();
        if !p.enabled {
            return false;
        }
        p.stack.push((name, Instant::now(), 0.0));
        true
    });
    Span(enabled, PhantomData)
}
impl Drop for Span {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        PROFILE.with(|p| {
            let mut p = p.borrow_mut();
            let (name, started, children) = p.stack.pop().expect("balanced profile spans");
            let total = started.elapsed().as_secs_f64();
            let entry = p.entries.entry(name).or_default();
            entry.0 += 1;
            entry.1 += total;
            entry.2 += (total - children).max(0.0);
            if let Some(parent) = p.stack.last_mut() {
                parent.2 += total;
            }
        });
    }
}
#[inline]
pub fn measure<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    let _span = span(name);
    f()
}
pub fn snapshot() -> serde_json::Value {
    PROFILE.with(|p| {
        let p = p.borrow();
        serde_json::json!({"enabled": p.enabled,
            "semantics": "thread-local wall clock; self excludes nested spans; inclusive totals overlap",
            "regions": p.entries.iter().map(|(name, (calls, total, own))|
                (*name, serde_json::json!({"calls": calls, "inclusive_seconds": total, "self_seconds": own})))
                .collect::<BTreeMap<_, _>>()})
    })
}
