// downloads.js — DownloadsPage class for /downloads.
// Depends on lms-core.js (HtmlUtils, LmsApi, PageState).

class DownloadsPage {
    constructor() {
        this.state = new PageState("downloads-container");
    }

    contentIcon(contentType) {
        const icons = {
            pdf:      "bi-file-pdf text-danger",
            document: "bi-file-earmark-text text-primary",
            video:    "bi-play-circle text-success",
            image:    "bi-image text-info",
            link:     "bi-link-45deg text-warning",
        };
        return icons[contentType] || "bi-file-earmark";
    }

    contentViewUrl(item) {
        if (item.content_type === "pdf" && item.content_url) {
            return `/pdf-viewer?url=${encodeURIComponent(item.content_url)}`;
        }
        if (item.content_url) {
            return item.content_url;
        }
        return `/module-content/${item.module_content_id}`;
    }

    renderDownloadItem(item, courseName, moduleName) {
        const icon     = this.contentIcon(item.content_type);
        const viewUrl  = this.contentViewUrl(item);
        const isExternal = item.content_url && !item.content_url.startsWith("/");
        const target   = isExternal ? ' target="_blank" rel="noopener"' : "";

        return `
            <div class="list-group-item d-flex align-items-center gap-3">
                <div class="flex-shrink-0 text-center" style="width:2rem;">
                    <i class="bi ${icon} fs-5"></i>
                </div>
                <div class="flex-grow-1 min-w-0">
                    <div class="fw-semibold text-truncate">${HtmlUtils.escape(item.title)}</div>
                    <div class="text-muted small">
                        ${HtmlUtils.escape(courseName)}
                        <i class="bi bi-chevron-right mx-1" style="font-size:.65rem;"></i>
                        ${HtmlUtils.escape(moduleName)}
                    </div>
                </div>
                <a href="${HtmlUtils.escape(viewUrl)}"${target}
                   class="btn btn-sm btn-outline-dark flex-shrink-0">
                    <i class="bi bi-box-arrow-up-right me-1"></i>Open
                </a>
            </div>`;
    }

    renderCourseSection(courseName, items) {
        return `
            <div class="mb-4">
                <h6 class="fw-bold text-uppercase text-muted small mb-2 px-1">
                    <i class="bi bi-journal-bookmark me-1"></i>${HtmlUtils.escape(courseName)}
                </h6>
                <div class="list-group list-group-flush border rounded-3">
                    ${items.map(({ item, moduleName }) =>
                        this.renderDownloadItem(item, courseName, moduleName)
                    ).join("")}
                </div>
            </div>`;
    }

    async load() {
        this.state.loading("Loading downloads...");

        try {
            const overview = await LmsApi.get("/api/my-courses/content-overview");

            if (!overview.length) {
                this.state.empty("You are not enrolled in any courses yet.", "bi-download");
                return;
            }

            const byCourse = {};
            for (const { course, modules } of overview) {
                for (const { module: mod, items } of modules || []) {
                    const downloadableItems = (items || []).filter(i =>
                        i.content_type === "pdf" || i.content_type === "document"
                    );

                    if (!downloadableItems.length) continue;

                    const key = course.course_id;
                    if (!byCourse[key]) {
                        byCourse[key] = {
                            courseName: course.name || `Course #${key}`,
                            rows: [],
                        };
                    }

                    for (const item of downloadableItems) {
                        byCourse[key].rows.push({
                            item,
                            moduleName: mod.title || `Module #${mod.module_id}`,
                        });
                    }
                }
            }

            const sections = Object.values(byCourse);

            if (!sections.length) {
                this.state.empty("No downloadable files have been posted for your courses yet.", "bi-file-earmark-x");
                return;
            }

            this.state.html(
                sections.map(s => this.renderCourseSection(s.courseName, s.rows)).join("")
            );
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load downloads. Please try again.");
        }
    }
}

document.addEventListener("DOMContentLoaded", () => {
    if (document.getElementById("downloads-container")) {
        new DownloadsPage().load();
    }
});
