use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOrganisationForm {
    pub org_name: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct OrganisationSignupForm {
    #[validate(length(min = 1, message = "Organisation name is required."))]
    pub org_name: String,

    #[validate(length(min = 1, message = "Organisation slug is required."))]
    pub org_slug: String,

    pub org_type: Option<String>,

    pub website_url: Option<String>,

    pub admin_first_name: Option<String>,

    pub admin_last_name: Option<String>,

    #[validate(email(message = "Please enter a valid admin email address."))]
    pub admin_email: Option<String>,

    pub admin_password: Option<String>,

    pub confirm_password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MassEnrollForm {
    /// List of user_ids to enroll into the org and assign a role
    pub user_ids: Vec<i32>,
    /// "Instructor" or "Student"
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct InviteInstructorForm {
    #[validate(email(message = "Please enter a valid instructor email address."))]
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssignCourseInstructorForm {
    pub instructor_id: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CourseInstructorDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct CourseInstructorCourseDto {
    pub course_id: i32,
    pub name: String,
    pub instructors: Vec<CourseInstructorDto>,
}

#[derive(Debug, Serialize)]
pub struct CourseInstructorSummaryDto {
    pub courses: Vec<CourseInstructorCourseDto>,
    pub instructors: Vec<CourseInstructorDto>,
}

#[derive(Debug, Serialize)]
pub struct OrgMemberDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub roles: Vec<String>,
}
