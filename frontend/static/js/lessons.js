// lessons.js — LessonsPage class for /lessons.
// Depends on lms-core.js (HtmlUtils, LmsApi, PageState).

class LessonsPage {
    constructor() {
        this.state = new PageState("lessons-container");
    }

    getContentIcon(contentType) {
        const icons = {
            video:    "bi-play-circle-fill text-success",
            pdf:      "bi-file-pdf-fill text-danger",
            document: "bi-file-earmark-text-fill text-primary",
            image:    "bi-image-fill text-info",
            link:     "bi-link-45deg text-warning",
        };
        return icons[contentType] || "bi-file-earmark-fill text-muted";
    }

    contentViewUrl(item) {
        if (item.content_type === "pdf" && item.content_url) {
            return `/pdf-viewer?url=${encodeURIComponent(item.content_url)}`;
        }
        if (item.content_type === "link" && item.content_url) {
            return item.content_url;
        }
        return `/module-content/${item.module_content_id}`;
    }

    isExternal(item) {
        return item.content_type === "link" && item.content_url && !item.content_url.startsWith("/");
    }

    renderContentItem(item) {
        const icon    = this.getContentIcon(item.content_type);
        const url     = this.contentViewUrl(item);
        const target  = this.isExternal(item) ? ' target="_blank" rel="noopener"' : "";
        const typeLabel = (item.content_type || "file").charAt(0).toUpperCase()
                        + (item.content_type || "file").slice(1);

        return `
            <a href="${HtmlUtils.escape(url)}"${target}
               class="list-group-item list-group-item-action d-flex align-items-center gap-3 py-2">
                <i class="bi ${icon} flex-shrink-0" style="font-size:1.2rem;"></i>
                <span class="flex-grow-1 text-truncate">${HtmlUtils.escape(item.title)}</span>
                <span class="badge bg-light text-dark border small flex-shrink-0">${typeLabel}</span>
            </a>`;
    }

    renderModuleSection(module, items) {
        if (!items.length) return "";

        return `
            <div class="mb-2">
                <div class="px-1 py-1 text-muted small fw-semibold">
                    <i class="bi bi-folder2-open me-1"></i>${HtmlUtils.escape(module.title || `Module #${module.module_id}`)}
                </div>
                <div class="list-group list-group-flush border rounded-3">
                    ${items.map(item => this.renderContentItem(item)).join("")}
                </div>
            </div>`;
    }

    renderCourseAccordion(courseGroup, index) {
        const collapseId    = `lessons-course-${index}`;
        const totalItems    = courseGroup.modules.reduce((sum, m) => sum + m.items.length, 0);
        const moduleSections = courseGroup.modules
            .map(m => this.renderModuleSection(m.module, m.items))
            .join("");

        const bodyContent = moduleSections.trim()
            ? moduleSections
            : `<p class="text-muted small py-2 px-2 mb-0">No content has been posted yet.</p>`;

        return `
            <div class="accordion-item border mb-2 rounded-3 overflow-hidden">
                <h2 class="accordion-header">
                    <button class="accordion-button ${index === 0 ? "" : "collapsed"} fw-semibold"
                            type="button"
                            data-bs-toggle="collapse"
                            data-bs-target="#${collapseId}"
                            aria-expanded="${index === 0 ? "true" : "false"}"
                            aria-controls="${collapseId}">
                        <i class="bi bi-journal-bookmark me-2 text-muted"></i>
                        ${HtmlUtils.escape(courseGroup.courseName)}
                        <span class="badge bg-secondary ms-2">${totalItems}</span>
                    </button>
                </h2>
                <div id="${collapseId}" class="accordion-collapse collapse ${index === 0 ? "show" : ""}">
                    <div class="accordion-body pt-2 pb-3">
                        ${bodyContent}
                    </div>
                </div>
            </div>`;
    }

    async load() {
        this.state.loading("Loading your lessons...");

        try {
            const courses = await LmsApi.get("/api/my-courses");

            if (!courses.length) {
                this.state.empty("You are not enrolled in any courses yet.", "bi-book");
                return;
            }

            // Round 1: all modules for all courses
            const moduleGroups = await Promise.all(
                courses.map(c =>
                    LmsApi.safeGet(`/api/modules/${c.course_id}`)
                        .then(modules => ({
                            course:  c,
                            modules: modules || [],
                        }))
                )
            );

            // Round 2: all content for all modules
            const contentData = await Promise.all(
                moduleGroups.flatMap(({ course, modules }) =>
                    modules.map(m =>
                        LmsApi.safeGet(`/api/module-content/${m.module_id}`)
                            .then(items => ({
                                courseId: course.course_id,
                                module:   m,
                                items:    items || [],
                            }))
                    )
                )
            );

            // Group by course
            const byCourse = {};
            for (const { courseId, module: mod, items } of contentData) {
                if (!byCourse[courseId]) {
                    const c = courses.find(x => x.course_id === courseId);
                    byCourse[courseId] = {
                        courseName: c?.name || `Course #${courseId}`,
                        modules:    [],
                    };
                }
                byCourse[courseId].modules.push({ module: mod, items });
            }

            const courseGroups = Object.values(byCourse);
            const hasContent   = courseGroups.some(g => g.modules.some(m => m.items.length > 0));

            if (!hasContent) {
                this.state.empty("No lessons have been posted for your courses yet.", "bi-book");
                return;
            }

            this.state.html(`
                <div class="accordion" id="lessons-accordion">
                    ${courseGroups.map((g, i) => this.renderCourseAccordion(g, i)).join("")}
                </div>`);
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load lessons. Please try again.");
        }
    }
}

document.addEventListener("DOMContentLoaded", () => {
    if (document.getElementById("lessons-container")) {
        new LessonsPage().load();
    }
});
