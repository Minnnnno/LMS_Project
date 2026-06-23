// course.js — CoursesPage class for the /courses page.
// Depends on lms-core.js (HtmlUtils, Course, LmsApi, PageState) and enrollment.js.

const COURSE_IMAGE_PRESETS = [
    { title: "Software Development", url: "/static/images/course-presets/software-development.jpg", width: 1600, height: 900 },
    { title: "Data Analytics", url: "/static/images/course-presets/data-analytics.jpg", width: 1600, height: 900 },
    { title: "Cybersecurity", url: "/static/images/course-presets/cybersecurity.jpg", width: 1600, height: 900 },
    { title: "Cloud Computing", url: "/static/images/course-presets/cloud-computing.jpg", width: 1600, height: 900 },
    { title: "Artificial Intelligence", url: "/static/images/course-presets/artificial-intelligence.jpg", width: 1600, height: 900 },
    { title: "Business Management", url: "/static/images/course-presets/business-management.jpg", width: 1600, height: 900 },
    { title: "Digital Marketing", url: "/static/images/course-presets/digital-marketing.jpg", width: 1600, height: 900 },
    { title: "Entrepreneurship", url: "/static/images/course-presets/entrepreneurship.jpg", width: 1600, height: 900 },
    { title: "Finance", url: "/static/images/course-presets/finance.jpg", width: 1600, height: 900 },
    { title: "Project Management", url: "/static/images/course-presets/project-management.jpg", width: 1600, height: 900 },
    { title: "Design Thinking", url: "/static/images/course-presets/design-thinking.jpg", width: 1600, height: 900 },
    { title: "UI/UX Design", url: "/static/images/course-presets/ui-ux-design.jpg", width: 1600, height: 900 },
    { title: "Photography", url: "/static/images/course-presets/photography.jpg", width: 1600, height: 900 },
    { title: "Healthcare", url: "/static/images/course-presets/healthcare.jpg", width: 1600, height: 900 },
    { title: "Education", url: "/static/images/course-presets/education.jpg", width: 1600, height: 900 },
    { title: "Communication", url: "/static/images/course-presets/communication.jpg", width: 1600, height: 900 },
    { title: "Leadership", url: "/static/images/course-presets/leadership.jpg", width: 1600, height: 900 },
    { title: "Languages", url: "/static/images/course-presets/languages.jpg", width: 1600, height: 900 },
    { title: "Engineering", url: "/static/images/course-presets/engineering.jpg", width: 1600, height: 900 },
    { title: "Hospitality", url: "/static/images/course-presets/hospitality.jpg", width: 1600, height: 900 },
];

const COURSE_IMAGE_RULES = {
    maxFileSizeBytes: 5 * 1024 * 1024,
    minWidth: 1200,
    minHeight: 675,
    targetRatio: 16 / 9,
    ratioTolerance: 0.08,
};

class CoursesPage {
    constructor() {
        this.organisationCourseIds = new Set();
        this.enrolledCourseIds     = new Set();
        this.completedCourseIds    = new Set();
        this.canManageOrg          = false;
        this.selectedPresetImage   = null;
        this.selectedImageFile     = null;
        this.selectedImageObjectUrl = null;
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
                       data-idle-label="${HtmlUtils.escape(course.enrollLabel)}"
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
        document.getElementById("create-course-visibility").value  = "public";
        document.getElementById("create-course-image-file").value  = "";
        document.getElementById("create-course-price").value       = "0";
        document.getElementById("create-course-currency").value    = "SGD";
        document.getElementById("create-course-paid").checked      = false;
        this.selectedPresetImage = null;
        this.selectedImageFile = null;
        this._clearImageObjectUrl();
        this._renderPresetImages();
        this._setImagePreview("");
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

    _clearImageObjectUrl() {
        if (this.selectedImageObjectUrl) {
            URL.revokeObjectURL(this.selectedImageObjectUrl);
            this.selectedImageObjectUrl = null;
        }
    }

    _setImagePreview(imageUrl) {
        const preview = document.getElementById("create-course-image-preview");

        if (!preview) return;

        if (!imageUrl) {
            preview.style.backgroundImage = "";
            preview.innerHTML = "<span>No image selected</span>";
            return;
        }

        preview.style.backgroundImage = `url('${imageUrl}')`;
        preview.innerHTML = "";
    }

    _renderPresetImages() {
        const grid = document.getElementById("create-course-preset-grid");

        if (!grid) return;

        grid.innerHTML = COURSE_IMAGE_PRESETS.map((preset, index) => `
            <button
                class="course-preset-option"
                type="button"
                data-preset-index="${index}"
                onclick="window.coursesPage.selectPresetImage(${index})"
            >
                <span class="course-preset-thumb" style="background-image: url('${HtmlUtils.escape(preset.url)}')"></span>
                <span>${HtmlUtils.escape(preset.title)}</span>
            </button>
        `).join("");
    }

    _syncPresetSelection() {
        document.querySelectorAll(".course-preset-option").forEach((button) => {
            const preset = COURSE_IMAGE_PRESETS[Number(button.dataset.presetIndex)];
            button.classList.toggle("selected", preset?.url === this.selectedPresetImage?.url);
        });
    }

    selectPresetImage(index) {
        const preset = COURSE_IMAGE_PRESETS[index];

        if (!preset || this._validateImageSize(preset.width, preset.height)) {
            this._setModalState(false, "Selected preset image does not fit the required cover size.", true);
            return;
        }

        this.selectedPresetImage = preset;
        this.selectedImageFile = null;
        this._clearImageObjectUrl();
        document.getElementById("create-course-image-file").value = "";
        this._setImagePreview(preset.url);
        this._syncPresetSelection();
        this._setModalState(false, "");
    }

    _validateImageSize(width, height) {
        if (width < COURSE_IMAGE_RULES.minWidth || height < COURSE_IMAGE_RULES.minHeight) {
            return `Image must be at least ${COURSE_IMAGE_RULES.minWidth} x ${COURSE_IMAGE_RULES.minHeight}.`;
        }

        if (Math.abs((width / height) - COURSE_IMAGE_RULES.targetRatio) > COURSE_IMAGE_RULES.ratioTolerance) {
            return "Image must be close to a 16:9 course cover shape.";
        }

        return "";
    }

    async _getImageDimensions(file) {
        const objectUrl = URL.createObjectURL(file);

        try {
            const image = new Image();
            const loaded = new Promise((resolve, reject) => {
                image.onload = () => resolve({
                    width: image.naturalWidth,
                    height: image.naturalHeight,
                });
                image.onerror = () => reject(new Error("Could not read image dimensions."));
            });

            image.src = objectUrl;
            return await loaded;
        } finally {
            URL.revokeObjectURL(objectUrl);
        }
    }

    async _validateUploadedImage(file) {
        if (!file.type.startsWith("image/")) {
            return "Please upload an image file.";
        }

        if (file.size > COURSE_IMAGE_RULES.maxFileSizeBytes) {
            return "Image must be 5 MB or smaller.";
        }

        const dimensions = await this._getImageDimensions(file);
        return this._validateImageSize(dimensions.width, dimensions.height);
    }

    async handleCourseImageFileChange(event) {
        const file = event.target.files?.[0] || null;

        if (!file) {
            this.selectedImageFile = null;
            this._clearImageObjectUrl();
            this._setImagePreview(this.selectedPresetImage?.url || "");
            return;
        }

        const validationMessage = await this._validateUploadedImage(file);

        if (validationMessage) {
            event.target.value = "";
            this.selectedImageFile = null;
            this._setModalState(false, validationMessage, true);
            return;
        }

        this.selectedImageFile = file;
        this.selectedPresetImage = null;
        this._syncPresetSelection();
        this._clearImageObjectUrl();
        this.selectedImageObjectUrl = URL.createObjectURL(file);
        this._setImagePreview(this.selectedImageObjectUrl);
        this._setModalState(false, "");
    }

    async _uploadCourseImage(file) {
        const formData = new FormData();
        formData.append("file", file);
        formData.append("folder", "lms/courses");

        const response = await axios.post("/api/cloudinary/upload", formData, {
            headers: {
                "Content-Type": "multipart/form-data",
            },
        });

        return response.data.secure_url;
    }

    async _getSelectedCourseImageUrl() {
        if (this.selectedImageFile) {
            this._setModalState(true, "Uploading course image...");
            return this._uploadCourseImage(this.selectedImageFile);
        }

        return this.selectedPresetImage?.url || null;
    }

    async createCourse() {
        const name        = document.getElementById("create-course-name").value.trim();
        const description = document.getElementById("create-course-description").value.trim();
        const visibility  = document.getElementById("create-course-visibility").value || "public";
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
            const imageUrl = await this._getSelectedCourseImageUrl();
            this._setModalState(true, "Creating course...");
            await LmsApi.post("/api/courses", {
                name,
                status: "draft",
                price,
                currency,
                is_paid: isPaid,
                description: description || null,
                background_image_url: imageUrl || null,
                visibility,
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
        const completedSection = document.getElementById("completed-courses-section");
        if (!section || !document.getElementById("enrolled-course-grid")) return;

        if (this.canManageOrg) {
            section.hidden = true;
            if (completedSection) completedSection.hidden = true;
            this.enrolledCourseIds = new Set();
            this.completedCourseIds = new Set();
            return;
        }

        try {
            const data = await LmsApi.get("/api/my-courses/completion-overview");
            const rows = data.filter(row => !this.organisationCourseIds.has(row.course.course_id));
            const completedCourses = rows
                .filter(row => row.completed)
                .map(row => new Course(row.course));
            const activeCourses = rows
                .filter(row => !row.completed)
                .map(row => new Course(row.course));

            this.enrolledCourseIds = new Set(rows.map(row => row.course.course_id));
            this.completedCourseIds = new Set(completedCourses.map(course => course.id));

            section.hidden = activeCourses.length === 0;
            if (activeCourses.length) {
                this.renderGrid("enrolled-course-grid", activeCourses);
            }

            if (completedSection) {
                completedSection.hidden = completedCourses.length === 0;
                if (completedCourses.length) {
                    this.renderGrid("completed-course-grid", completedCourses);
                }
            }
        } catch (error) {
            this.enrolledCourseIds = new Set();
            this.completedCourseIds = new Set();
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
                button.disabled = false;
                button.textContent = originalText;
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
        document.getElementById("create-course-image-file")
            ?.addEventListener("change", (event) => this.handleCourseImageFileChange(event));

        await this.loadOrganisationCourses();

        if (this.isManagedOnly) {
            document.getElementById("enrolled-courses-section")?.setAttribute("hidden", "");
            document.getElementById("all-courses-section")?.setAttribute("hidden", "");
            return;
        }

        await this.loadEnrolledCourses();
        await this.loadCourses();
    }

    resetEnrollmentButtons() {
        document.querySelectorAll(".course-card-action[data-idle-label]").forEach(button => {
            button.disabled = false;
            button.textContent = button.dataset.idleLabel;
        });
    }

    async refreshCourseLists() {
        await this.loadOrganisationCourses();

        if (this.isManagedOnly) {
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

window.addEventListener("pageshow", (event) => {
    window.coursesPage?.resetEnrollmentButtons();

    if (event.persisted) {
        window.coursesPage?.refreshCourseLists();
    }
});
