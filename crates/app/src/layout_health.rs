use anyhow::Result;
use crate::layout_preflight::LayoutPreflightService;
use domain::layout::VerifyType;

pub struct LayoutHealthService;

impl LayoutHealthService {
    pub fn run_health_checks(health_checks: &[VerifyType]) -> Result<Option<String>> {
        for (index, check) in health_checks.iter().enumerate() {
            match LayoutPreflightService::run_verify_check(check) {
                Ok(true) => {
                    log::info!("Health check {} passed", index + 1);
                }
                Ok(false) => {
                    return Ok(Some(format!(
                        "Health check {}/{} failed after apply",
                        index + 1,
                        health_checks.len()
                    )));
                }
                Err(error) => {
                    return Ok(Some(format!(
                        "Health check {}/{} errored after apply: {}",
                        index + 1,
                        health_checks.len(),
                        error
                    )));
                }
            }
        }

        Ok(None)
    }
}
