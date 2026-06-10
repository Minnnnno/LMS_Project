let organisationCourseIds = new Set();
let enrolledCourseIds = new Set();

function goToCourse(courseId) {
    window.location.href = "/course/" + courseId;
}

function renderCourseGrid(courseGrid, courses, options = {}) {
    courseGrid.innerHTML = "";

    if (!courses.length) {
        courseGrid.innerHTML = `<p class="course-empty">No training programmes available.</p>`;
        return;
    }

    courses.forEach((course) => {
        const manageBadge = options.manage
            ? `<span class="course-badge">Manage</span>`
            : "";

        courseGrid.innerHTML += `
            <div class="modern-course-card" onclick="goToCourse(${course.course_id})">
                <div class="course-image" style="background-image: url('${course.background_image_url || ""}')">
                    ${manageBadge}
                </div>
                <div class="course-content">
                    <h3 class="course-title">${course.name || "Untitled course"}</h3>
                    <p class="course-description">${course.description || "No description"}</p>
                </div>
            </div>
        `;
    });
}

async function loadOrganisationCourses() {
    const section = document.getElementById("organisation-courses-section");
    const courseGrid = document.getElementById("organisation-course-grid");

    if (!section || !courseGrid) {
        return;
    }

    try {
        const response = await axios.get("/api/courses/organisation");
        const courses = response.data;

        organisationCourseIds = new Set(courses.map((course) => course.course_id));
        section.hidden = false;
        renderCourseGrid(courseGrid, courses, { manage: true });
    } catch (error) {
        organisationCourseIds = new Set();

        if (error.response?.status !== 401 && error.response?.status !== 403) {
            console.error("Failed to load organisation training programmes:", error);
        }
    }
}

async function loadCourses() {
    try {
        const response = await axios.get("/api/courses");
        const courseGrid = document.getElementById("course-grid");
        const allCoursesSection = document.getElementById("all-courses-section");
        const courses = response.data.filter((course) =>
            !organisationCourseIds.has(course.course_id)
            && !enrolledCourseIds.has(course.course_id)
        );

        if (allCoursesSection) {
            allCoursesSection.hidden = organisationCourseIds.size > 0 && courses.length === 0;
        }

        renderCourseGrid(courseGrid, courses);
    } catch (error) {
        console.error("Failed to load training programmes:", error);
    }
}

async function loadEnrolledCourses() {
    const section = document.getElementById("enrolled-courses-section");
    const courseGrid = document.getElementById("enrolled-course-grid");

    if (!section || !courseGrid) {
        return;
    }

    try {
        const response = await axios.get("/api/my-courses");
        const courses = response.data.filter((course) => !organisationCourseIds.has(course.course_id));

        enrolledCourseIds = new Set(courses.map((course) => course.course_id));

        if (courses.length) {
            section.hidden = false;
            renderCourseGrid(courseGrid, courses);
        }
    } catch (error) {
        enrolledCourseIds = new Set();

        if (error.response?.status !== 401) {
            console.error("Failed to load enrolled training programmes:", error);
        }
    }
}

async function initCoursesPage() {
    await loadOrganisationCourses();
    await loadEnrolledCourses();
    await loadCourses();
}

initCoursesPage();
