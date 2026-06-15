use serde::Serialize;

#[derive(Serialize)]
pub struct CourseProgress {
    pub completed_modules: u64,
    pub total_modules: u64,
    pub progress_percent: u8,
}

#[derive(Serialize)]
pub struct ModuleProgress {
    pub opened: bool,
    pub progress_percent: u8,
}

#[derive(Serialize)]
pub struct CourseModuleProgress {
    pub module_id: i32,
    pub opened: bool,
    pub progress_percent: u8,
}
