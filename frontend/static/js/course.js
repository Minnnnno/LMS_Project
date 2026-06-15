// course.js — CoursesPage class for the /courses page.
// Depends on lms-core.js (HtmlUtils, Course, LmsApi, PageState) and enrollment.js.

class CoursesPage {
    constructor() {
        this.organisationCourseIds = new Set();
        this.enrolledCourseIds     = new Set();
        this.canManageOrg          = false;
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    get isManagedOnly() {
        return document.getElementById("organisation-courses-section")
            ?.dataset.managedOnly === "true";
    }

    renderGrid(containerId, courses, options = {}) {
        const el = document.getElementById(containerId);
        if (!el) return;

        if (!courses.length) {
            el.innerHTML = `<p class="course-empty">No courses available.</p>`;
            return;
        }

        el.innerHTML = courses.map(course => {
            const manageBadge = options.manage
                ? `<span class="course-badge">Manage</span>`
                : "";
            const actionButton = options.showEnrollmentAction
                ? `<button class="course-card-action" type="button"
                       onclick="window.coursesPage.handleEnrollmentAction(event, ${course.id})">
                       ${HtmlUtils.escape(course.enrollLabel)}
                   </button>`
                : "";
            const cardFooter = options.showEnrollmentAction
                ? `<div class="course-card-footer">
                       <span class="course-card-price">${HtmlUtils.escape(course.formattedPrice)}</span>
                       ${actionButton}
                   </div>`
                : "";
            const cardClick = options.showEnrollmentAction
                ? ""
                : `onclick="window.location.href='/course/${course.id}'"`;
            const cardClass = options.showEnrollmentAction
                ? "modern-course-card course-action-card"
                : "modern-course-card";

            return `
                <div class="${cardClass}" ${cardClick}>
                    <div class="course-image" style="background-image: url('${HtmlUtils.escape(course.imageUrl)}')">
                        ${manageBadge}
                    </div>
                    <div class="course-content">
                        <h3 class="course-title">${HtmlUtils.escape(course.name)}</h3>
                        <p class="course-description">${HtmlUtils.escape(course.description)}</p>
                        ${cardFooter}
                    </div>
                </div>`;
        }).join("");
    }

    // ---------------------------------------------------------------------------
    // Create-course modal
    // ---------------------------------------------------------------------------

    openModal() {
        document.getElementById("create-course-name").value        = "";
        document.getElementById("create-course-description").value = "";
        document.getElementById("create-course-image").value       = "";
        document.getElementById("create-course-price").value       = "0";
        document.getElementById("create-course-currency").value    = "SGD";
        document.getElementById("create-course-paid").checked      = false;
        this._setModalState(false, "");
        document.getElementById("create-course-modal").style.display = "flex";
    }

    closeModal() {
        document.getElementById("create-course-modal").style.display = "none";
    }

    _setModalState(isSaving, message = "", isError = false) {
        const saveBtn  = document.getElementById("save-create-course-btn");
        const closeBtn = document.getElementById("close-create-course-btn");
        const status   = document.getElementById("create-course-status");

        if (saveBtn)  { saveBtn.disabled = isSaving; saveBtn.textContent = isSaving ? "Creating..." : "Create"; }
        if (closeBtn) { closeBtn.disabled = isSaving; }
        if (status)   { status.textContent = message; status.className = isError ? "course-form-status error" : "course-form-status"; }
    }

    async createCourse() {
        const name        = document.getElementById("create-course-name").value.trim();
        const description = document.getElementById("create-course-description").value.trim();
        const imageUrl    = document.getElementById("create-course-image").value.trim();
        const price       = Number(document.getElementById("create-course-price").value || 0);
        const currency    = document.getElementById("create-course-currency").value || "SGD";
        const isPaid      = document.getElementById("create-course-paid").checked;

        if (!name) {
            this._setModalState(false, "Please enter a course name.", true);
            return;
        }
        if (!Number.isFinite(price) || price < 0) {
            this._setModalState(false, "Please enter a valid price.", true);
            return;
        }

        try {
            this._setModalState(true, "Creating course...");
            await LmsApi.post("/api/courses", {
                name,
                status: "draft",
                price,
                currency,
                is_paid: isPaid,
                description: description || null,
                background_image_url: imageUrl || null,
            });
            this.closeModal();
            await this.loadOrganisationCourses();
            await this.loadCourses();
        } catch (error) {
            if (error.response?.status === 401) {
                window.location.href = "/login";
                return;
            }
            this._setModalState(false, error.response?.data || "Failed to create course.", true);
        }
    }

    // ---------------------------------------------------------------------------
    // Data loaders
    // ---------------------------------------------------------------------------

    async loadOrganisationCourses() {
        const section   = document.getElementById("organisation-courses-section");
        if (!section || !document.getElementById("organisation-course-grid")) return;

        try {
            const data    = await LmsApi.get("/api/courses/organisation");
            const courses = data.map(d => new Course(d));

            this.organisationCourseIds = new Set(courses.map(c => c.id));
            this.canManageOrg = true;
            section.hidden = false;

            const createBtn = document.getElementById("open-create-course-btn");
            if (createBtn) createBtn.hidden = createBtn.dataset.canCreate !== "true";

            this.renderGrid("organisation-course-grid", courses, { manage: true });
        } catch (error) {
            this.organisationCourseIds = new Set();
            this.canManageOrg = false;

            const createBtn = document.getElementById("open-create-course-btn");
            if (createBtn) createBtn.hidden = true;

            if (error.response?.status !== 401 && error.response?.status !== 403) {
                console.error("Failed to load organisation courses:", error);
            }
        }
    }

    async loadEnrolledCourses() {
        if (this.isManagedOnly) return;

        const section = document.getElementById("enrolled-courses-section");
        if (!section || !document.getElementById("enrolled-course-grid")) return;

        if (this.canManageOrg) {
            section.hidden = true;
            this.enrolledCourseIds = new Set();
            return;
        }

        try {
            const data    = await LmsApi.get("/api/my-courses");
            const courses = data
                .filter(c => !this.organisationCourseIds.has(c.course_id))
                .map(c => new Course(c));

            this.enrolledCourseIds = new Set(courses.map(c => c.id));

            if (courses.length) {
                section.hidden = false;
                this.renderGrid("enrolled-course-grid", courses);
            }
        } catch (error) {
            this.enrolledCourseIds = new Set();
            if (error.response?.status !== 401) {
                console.error("Failed to load enrolled courses:", error);
            }
        }
    }

    async loadCourses() {
        if (this.isManagedOnly) return;

        try {
            const data    = await LmsApi.get("/api/courses");
            const courses = data
                .filter(c =>
                    !this.organisationCourseIds.has(c.course_id) &&
                    !this.enrolledCourseIds.has(c.course_id)
                )
                .map(c => new Course(c));

            const allSection = document.getElementById("all-courses-section");
            if (allSection) {
                allSection.hidden = this.organisationCourseIds.size > 0 && courses.length === 0;
            }

            this.renderGrid("course-grid", courses, { showEnrollmentAction: true });
        } catch (error) {
            console.error("Failed to load courses:", error);
        }
    }

    // ---------------------------------------------------------------------------
    // Enrollment action handler (called from inline onclick)
    // ---------------------------------------------------------------------------

    async handleEnrollmentAction(event, courseId) {
        event.stopPropagation();

        const button      = event.currentTarget;
        const originalText = button.textContent.trim();
        const isPaid      = originalText === "Buy Course";

        button.disabled    = true;
        button.textContent = isPaid ? "Opening checkout..." : "Enrolling...";

        try {
            if (isPaid) {
                await EnrollmentHelper.startCheckout(courseId);
                return;
            }
            await EnrollmentHelper.enrollFree(courseId);
            this.enrolledCourseIds.add(courseId);
            button.closest(".modern-course-card")?.remove();
            window.location.href = `/course/${courseId}`;
        } catch (error) {
            if (error.response?.status === 401) {
                window.location.href = "/login";
                return;
            }
            button.disabled    = false;
            button.textContent = originalText;
            alert(error.response?.data || "Unable to process this course right now.");
        }
    }

    // ---------------------------------------------------------------------------
    // Boot
    // ---------------------------------------------------------------------------

    async init() {
        document.getElementById("open-create-course-btn")
            ?.addEventListener("click", () => this.openModal());
        document.getElementById("close-create-course-btn")
            ?.addEventListener("click", () => this.closeModal());
        document.getElementById("save-create-course-btn")
            ?.addEventListener("click", () => this.createCourse());

        await this.loadOrganisationCourses();

        if (this.isManagedOnly) {
            document.getElementById("enrolled-courses-section")?.setAttribute("hidden", "");
            document.getElementById("all-courses-section")?.setAttribute("hidden", "");
            return;
        }

        await this.loadEnrolledCourses();
        await this.loadCourses();
    }
}

document.addEventListener("DOMContentLoaded", () => {
    window.coursesPage = new CoursesPage();
    window.coursesPage.init();
});
