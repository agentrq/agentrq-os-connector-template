//! Example: a workflow connector.
//!
//! A workflow connector declares no tools: it holds logic the host runs step
//! by step. The manifest's `workflow` block names the `start` step and, for
//! each step, the steps its ok path and its error path may lead to. The host
//! checks that every ok/err target names a declared step and that every step
//! is reachable from the start — [`validate`] mirrors that check so it can be
//! unit-tested.

/// A workflow step: the step function plus where its paths may lead.
pub struct Step {
    /// Step name: lowercase letters, digits and underscores.
    pub name: &'static str,
    /// Steps the ok path may lead to.
    pub ok: &'static [&'static str],
    /// Steps the error path may lead to.
    pub err: &'static [&'static str],
}

/// The steps of the example workflow.
#[must_use]
pub fn steps() -> Vec<Step> {
    vec![
        Step {
            name: "fetch_status",
            ok: &["summarize"],
            err: &["report_error"],
        },
        Step {
            name: "summarize",
            ok: &["done"],
            err: &["report_error"],
        },
        Step {
            name: "report_error",
            ok: &["done"],
            err: &[],
        },
        Step {
            name: "done",
            ok: &[],
            err: &[],
        },
    ]
}

/// The step a run starts at.
#[must_use]
pub fn start_step() -> &'static str {
    "fetch_status"
}

/// Checks that every ok/err target names a declared step, the way the host
/// does when it loads the manifest.
///
/// # Errors
///
/// Returns an error naming the first step that points at an undeclared step.
pub fn validate(steps: &[Step]) -> Result<(), String> {
    for step in steps {
        for target in step.ok.iter().chain(step.err.iter()) {
            if !steps.iter().any(|s| s.name == *target) {
                return Err(format!(
                    "step '{}' points at unknown step '{target}'",
                    step.name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_fetch_status() {
        assert_eq!(start_step(), "fetch_status");
    }

    #[test]
    fn declares_four_steps() {
        let names: Vec<_> = steps().iter().map(|s| s.name).collect();
        assert_eq!(names, ["fetch_status", "summarize", "report_error", "done"]);
    }

    #[test]
    fn the_example_graph_is_valid() {
        assert_eq!(validate(&steps()), Ok(()));
    }

    #[test]
    fn accepts_an_empty_graph() {
        assert_eq!(validate(&[]), Ok(()));
    }

    #[test]
    fn rejects_a_dangling_ok_target() {
        let steps = [Step {
            name: "only",
            ok: &["missing"],
            err: &[],
        }];
        assert_eq!(
            validate(&steps),
            Err("step 'only' points at unknown step 'missing'".to_string())
        );
    }

    #[test]
    fn rejects_a_dangling_err_target() {
        let steps = [Step {
            name: "only",
            ok: &[],
            err: &["missing"],
        }];
        assert_eq!(
            validate(&steps),
            Err("step 'only' points at unknown step 'missing'".to_string())
        );
    }
}
