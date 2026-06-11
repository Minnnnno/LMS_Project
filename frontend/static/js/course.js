let organisationCourseIds = new Set();
let enrolledCourseIds = new Set();

function goToCourse(courseId) {
    window.location.href = "/course/" + courseId;
}

function getCoursePriceCents(course) {
    if (Number.isFinite(Number(course.price))) {
        const decimalPriceCents = Math.round(Number(course.price) * 100);

        if (decimalPriceCents > 0) {
            return decimalPriceCents;
        }
    }

    if (Number.isFinite(Number(course.price_cents))) {
        return Number(course.price_cents);
    }

    return null;
}

function isPaidCourse(course) {
    const priceCents = getCoursePriceCents(course);
    return Boolean(course.is_paid) || (priceCents !== null && priceCents > 0);
}

function getCourseActionLabel(course) {
    return isPaidCourse(course) ? "Buy Course" : "Enroll Now";
}

function formatCoursePrice(course) {
    if (!isPaidCourse(course)) {
        return "Free";
    }

    const priceCents = getCoursePriceCents(course);
    const currency = course.currency || "SGD";

    if (priceCents === null) {
        return "Price unavailable";
    }

    return new Intl.NumberFormat("en-SG", {
        style: "currency",
        currency,
    }).format(priceCents / 100);
}

function renderCourseGrid(courseGrid, courses, options = {}) {
    courseGrid.innerHTML = "";

    if (!courses.length) {
        courseGrid.innerHTML = `<p class="course-empty">No courses available.</p>`;
        return;
    }

    courses.forEach((course) => {
        const manageBadge = options.manage
            ? `<span class="course-badge">Manage</span>`
            : "";
        const actionButton = options.showEnrollmentAction
            ? `<button class="course-card-action" type="button" onclick="handleCourseEnrollmentAction(event, ${course.course_id})">${getCourseActionLabel(course)}</button>`
            : "";
        const cardFooter = options.showEnrollmentAction
            ? `
                <div class="course-card-footer">
                    <span class="course-card-price">${formatCoursePrice(course)}</span>
                    ${actionButton}
                </div>
            `
            : "";
        const cardClick = options.showEnrollmentAction
            ? ""
            : `onclick="goToCourse(${course.course_id})"`;
        const cardClass = options.showEnrollmentAction
            ? "modern-course-card course-action-card"
            : "modern-course-card";

        courseGrid.innerHTML += `
            <div class="${cardClass}" ${cardClick}>
                <div class="course-image" style="background-image: url('${course.background_image_url || ""}')">
                    ${manageBadge}
                </div>
                <div class="course-content">
                    <h3 class="course-title">${course.name || "Untitled course"}</h3>
                    <p class="course-description">${course.description || "No description"}</p>
                    ${cardFooter}
                </div>
            </div>
        `;
    });
}

async function handleCourseEnrollmentAction(event, courseId) {
    event.stopPropagation();

    const button = event.currentTarget;
    const course = button.closest(".modern-course-card");
    const originalText = button.textContent;
    const isPaid = originalText === "Buy Course";

    button.disabled = true;
    button.textContent = isPaid ? "Opening checkout..." : "Enrolling...";

    try {
        if (isPaid) {
            const response = await axios.post(`/api/courses/${courseId}/checkout`);
            window.location.href = response.data.checkout_url;
            return;
        }

        await axios.post(`/api/courses/${courseId}/enroll`);
        enrolledCourseIds.add(courseId);
        course?.remove();
        window.location.href = `/course/${courseId}`;
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        button.disabled = false;
        button.textContent = originalText;
        alert(error.response?.data || "Unable to process this course right now.");
    }
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
            console.error("Failed to load organisation courses:", error);
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

        renderCourseGrid(courseGrid, courses, { showEnrollmentAction: true });
    } catch (error) {
        console.error("Failed to load courses:", error);
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
            console.error("Failed to load enrolled courses:", error);
        }
    }
}

async function initCoursesPage() {
    await loadOrganisationCourses();
    await loadEnrolledCourses();
    await loadCourses();
}

initCoursesPage();
