class CategoryPreferencesModal {
    constructor() {
        this.modalElement = document.getElementById("category-preferences-modal");
        this.modal = this.modalElement ? new bootstrap.Modal(this.modalElement) : null;
        this.titleElement = document.getElementById("category-preferences-title");
        this.optionsElement = document.getElementById("category-preferences-options");
        this.recommendationsElement = document.getElementById("category-recommendations");
        this.pickerElement = document.getElementById("category-preferences-picker");
        this.alertElement = document.getElementById("category-preferences-alert");
        this.saveButton = document.getElementById("save-category-preferences-btn");
        this.dismissed = false;
        this.recommendedCourses = new Map();
        this.dismissalKey = this.modalElement?.dataset.userId
            ? `lms_category_preferences_dismissed_${this.modalElement.dataset.userId}`
            : "lms_category_preferences_dismissed";
    }

    async init() {
        if (!this.modal || !this.saveButton) return;

        this.saveButton.addEventListener("click", () => this.savePreferences());
        this.recommendationsElement?.addEventListener("click", (event) => this.handleEnrollment(event));
        this.modalElement.addEventListener("hide.bs.modal", () => {
            if (!this.dismissed && !this.recommendationsElement?.classList.contains("d-none")) {
                return;
            }
            localStorage.setItem(this.dismissalKey, "true");
        });

        try {
            const payload = await LmsApi.get("/api/course-preferences");

            if (payload.should_prompt && payload.categories?.length) {
                if (localStorage.getItem(this.dismissalKey) === "true") {
                    return;
                }
                this.renderCategories(payload.categories);
                this.modal.show();
                return;
            }

            if (payload.recommended_courses?.length) {
                this.renderRecommendations(payload.recommended_courses);
                this.modal.show();
            }
        } catch (error) {
            if (error.response?.status !== 401 && error.response?.status !== 403) {
                console.error("Failed to load course preferences:", error);
            }
        }
    }

    renderCategories(categories) {
        this.clearAlert();
        if (this.titleElement) {
            this.titleElement.textContent = "Choose course categories";
        }
        this.pickerElement?.classList.remove("d-none");
        this.recommendationsElement?.classList.add("d-none");
        if (this.saveButton) {
            this.saveButton.hidden = false;
            this.saveButton.disabled = false;
            this.saveButton.textContent = "Show courses";
        }

        if (!this.optionsElement) return;

        this.optionsElement.innerHTML = categories.map(category => `
            <label class="category-preference-option">
                <input class="category-preference-input" type="checkbox" value="${HtmlUtils.escape(category)}">
                <span class="category-preference-emoji" aria-hidden="true">${HtmlUtils.escape(this.categoryEmoji(category))}</span>
                <span class="category-preference-name">${HtmlUtils.escape(category)}</span>
                <i class="bi bi-check2 category-preference-check" aria-hidden="true"></i>
            </label>
        `).join("");
    }

    renderRecommendations(courses) {
        this.clearAlert();
        if (this.titleElement) {
            this.titleElement.textContent = "Here are some courses we recommend";
        }
        this.pickerElement?.classList.add("d-none");
        this.recommendationsElement?.classList.remove("d-none");
        if (this.saveButton) {
            this.saveButton.hidden = true;
        }

        if (!this.recommendationsElement) return;

        this.recommendedCourses = new Map();

        if (!courses.length) {
            this.recommendationsElement.innerHTML = `<p class="category-recommendation-empty">No courses are available for those categories yet.</p>`;
            return;
        }

        this.recommendationsElement.innerHTML = courses.map(rawCourse => {
            const course = new Course(rawCourse);
            this.recommendedCourses.set(course.id, course);
            return `
                <article class="category-recommendation-card">
                    <span class="category-recommendation-image" style="background-image: url('${HtmlUtils.escape(course.imageUrl)}')"></span>
                    <span class="category-recommendation-body">
                        <strong>${HtmlUtils.escape(course.name)}</strong>
                        <span>${HtmlUtils.escape(course.description)}</span>
                    </span>
                    <span class="category-recommendation-footer">
                        <span class="category-recommendation-price">${HtmlUtils.escape(course.formattedPrice)}</span>
                        <button class="btn btn-dark btn-sm category-recommendation-enroll" type="button" data-course-id="${course.id}">
                            Enroll Now
                        </button>
                    </span>
                </article>
            `;
        }).join("");
    }

    categoryEmoji(category) {
        return {
            STEM: "🔬",
            Lifestyle: "🌿",
            Finance: "💰",
            Technology: "💻",
        }[category] || "📚";
    }

    selectedCategories() {
        return Array.from(this.optionsElement?.querySelectorAll("input:checked") || [])
            .map(input => input.value);
    }

    async handleEnrollment(event) {
        const button = event.target.closest(".category-recommendation-enroll");
        if (!button) return;

        const courseId = Number(button.dataset.courseId);
        const course = this.recommendedCourses.get(courseId);
        if (!course) return;

        try {
            this.clearAlert();
            button.disabled = true;
            button.textContent = course.isPaid ? "Opening checkout..." : "Enrolling...";

            if (course.isPaid) {
                await EnrollmentHelper.startCheckout(courseId);
                return;
            }

            await EnrollmentHelper.enrollFree(courseId);
            window.location.href = `/course/${courseId}`;
        } catch (error) {
            button.disabled = false;
            button.textContent = "Enroll Now";

            if (error.response?.status === 401) {
                window.location.href = "/login";
                return;
            }

            this.showAlert(error.response?.data || "Unable to enroll in this course right now.");
        }
    }

    async savePreferences() {
        const categories = this.selectedCategories();

        if (!categories.length) {
            this.showAlert("Select at least one category.");
            return;
        }

        try {
            this.clearAlert();
            this.saveButton.disabled = true;
            this.saveButton.textContent = "Saving...";
            const payload = await LmsApi.post("/api/course-preferences", { categories });
            localStorage.removeItem(this.dismissalKey);
            this.dismissed = true;
            this.renderRecommendations(payload.recommended_courses || []);
        } catch (error) {
            this.saveButton.disabled = false;
            this.saveButton.textContent = "Show courses";
            this.showAlert(error.response?.data || "Unable to save your preferences right now.");
        }
    }

    showAlert(message) {
        if (!this.alertElement) return;
        this.alertElement.textContent = message;
        this.alertElement.classList.remove("d-none");
    }

    clearAlert() {
        if (!this.alertElement) return;
        this.alertElement.textContent = "";
        this.alertElement.classList.add("d-none");
    }
}

document.addEventListener("DOMContentLoaded", () => {
    new CategoryPreferencesModal().init();
});
