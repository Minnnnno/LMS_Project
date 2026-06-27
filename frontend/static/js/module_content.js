const pathParts = window.location.pathname.split("/");
const moduleId = pathParts[2];
let currentContents = [];
let currentEditingContentId = null;
let canManageContent = false;
let statusMessageTimer = null;
let moduleOpened = false;
let openedContentIds = loadOpenedContentIds();

function normalizeEnumValue(value) {
    return value ? String(value).toLowerCase() : "";
}

function getOpenedContentStorageKey() {
    return `skillup-opened-module-content-${moduleId}`;
}

function loadOpenedContentIds() {
    try {
        const storedIds = JSON.parse(localStorage.getItem(getOpenedContentStorageKey()) || "[]");
        return new Set(Array.isArray(storedIds) ? storedIds.map(Number) : []);
    } catch (_error) {
        return new Set();
    }
}

function saveOpenedContentIds() {
    try {
        localStorage.setItem(getOpenedContentStorageKey(), JSON.stringify([...openedContentIds]));
    } catch (_error) {
        // Ignore storage failures; the backend module progress still records completion.
    }
}

function markContentOpened(contentId) {
    if (!Number.isFinite(Number(contentId))) {
        return;
    }

    openedContentIds.add(Number(contentId));
    saveOpenedContentIds();
}

async function loadManageAccess() {
    try {
        const response = await axios.get(`/api/module-content/${moduleId}/manage-access`);
        canManageContent = Boolean(response.data.can_manage);
        document.getElementById("add-content-btn").style.display = canManageContent ? "inline-flex" : "none";

        if (canManageContent) {
            hideModuleProgress();
        }
    } catch (error) {
        canManageContent = false;
        console.error("Failed to load module content access:", error);
    }
}

async function loadModuleContent() {
    const contentList = document.getElementById("content-list");

    try {
        const response = await axios.get("/api/module-content/" + moduleId);
        const contents = Array.isArray(response.data)
            ? response.data
            : [response.data];

        currentContents = contents.sort((first, second) => first.position - second.position);
        contentList.innerHTML = "";

        if (!currentContents.length) {
            contentList.innerHTML = "<p>No content available for this module.</p>";
            return;
        }

        currentContents.forEach((content) => {
            const actions = canManageContent
                ? `
                    <div class="content-actions">
                        <button class="content-action-btn edit-btn" onclick="editContent(event, ${content.module_content_id})">Edit</button>
                        <button class="content-action-btn delete-btn" onclick="deleteContent(event, ${content.module_content_id})">Delete</button>
                    </div>
                `
                : "";
            const contentId = Number(content.module_content_id);
            const contentOpened = openedContentIds.has(contentId);
            const encodedContentUrl = encodeURIComponent(content.content_url || "");
            const openedTick = !canManageContent && contentOpened
                ? '<span class="content-opened-tick" aria-label="Opened"><i class="bi bi-check-lg" aria-hidden="true"></i></span>'
                : "";

            contentList.innerHTML += `
                <div class="module-row ${contentOpened ? "opened" : ""}" onclick="openContent(${contentId}, '${encodedContentUrl}')">
                    <div>
                        <span class="content-title">${content.title}</span>
                        <span class="content-meta">${content.content_type}${content.content_category ? ` · ${content.content_category}` : ""}</span>
                    </div>
                    ${actions}
                    ${openedTick}
                    <span class="module-arrow">&rsaquo;</span>
                </div>
            `;
        });
    } catch (error) {
        currentContents = [];
        contentList.innerHTML = "<p>No content available for this module.</p>";
        console.error("Failed to load module content:", error);
    }
}

function hideModuleProgress() {
    const progressCard = document.getElementById("module-progress-card");
    moduleOpened = false;

    if (progressCard) {
        progressCard.hidden = true;
    }
}

function renderModuleProgress(progress) {
    const progressCard = document.getElementById("module-progress-card");
    const progressSummary = document.getElementById("module-progress-summary");
    const progressPercent = document.getElementById("module-progress-percent");
    const progressFill = document.getElementById("module-progress-fill");

    if (!progressCard || !progressSummary || !progressPercent || !progressFill) {
        return;
    }

    const opened = Boolean(progress.opened);
    const percent = opened ? 100 : Math.max(0, Math.min(100, Number(progress.progress_percent || 0)));
    moduleOpened = opened;

    progressSummary.textContent = opened ? "Opened" : "Not opened yet";
    progressPercent.textContent = `${percent}%`;
    progressFill.style.width = `${percent}%`;
    progressCard.hidden = false;
}

async function loadModuleProgress() {
    if (canManageContent) {
        hideModuleProgress();
        return;
    }

    try {
        const response = await axios.get(`/api/module-content/${moduleId}/progress`);
        renderModuleProgress(response.data || {});
    } catch (error) {
        hideModuleProgress();

        if (![401, 403, 404].includes(error.response?.status)) {
            console.error("Failed to load module progress:", error);
        }
    }
}

async function markModuleOpened() {
    if (canManageContent) {
        return;
    }

    try {
        const response = await axios.post(`/api/module-content/${moduleId}/progress`);
        renderModuleProgress(response.data || {
            opened: true,
            progress_percent: 100,
        });
        sessionStorage.setItem("skillup-course-progress-dirty", "1");
    } catch (error) {
        if (![401, 403].includes(error.response?.status)) {
            console.error("Failed to update module progress:", error);
        }
    }
}

async function openContent(contentId, encodedContentUrl) {
    if (!encodedContentUrl) {
        return;
    }

    markContentOpened(contentId);
    loadModuleContent();
    await markModuleOpened();
    window.location.href = `/pdf-viewer?url=${encodedContentUrl}`;
}

function openContentModal(content = null) {
    currentEditingContentId = content?.module_content_id || null;

    document.getElementById("content-modal-title").textContent = content ? "Edit Content" : "Add Content";
    document.getElementById("content-title-input").value = content?.title || "";
    document.getElementById("content-title-input").placeholder = content?.title || "Lecture slides";
    document.getElementById("content-type-input").value = normalizeEnumValue(content?.content_type) || "pdf";
    document.getElementById("content-category-input").value = normalizeEnumValue(content?.content_category);
    document.getElementById("content-file-input").value = "";
    document.getElementById("content-position-input").value = content?.position || currentContents.length + 1;
    document.getElementById("content-position-input").placeholder =
        String(content?.position || currentContents.length + 1);
    document.getElementById("content-modal").style.display = "flex";
}

function closeContentModal() {
    currentEditingContentId = null;
    document.getElementById("content-title-input").value = "";
    document.getElementById("content-title-input").placeholder = "Lecture slides";
    document.getElementById("content-file-input").value = "";
    document.getElementById("content-position-input").value = "";
    document.getElementById("content-position-input").placeholder = "1";
    document.getElementById("content-modal").style.display = "none";
}

function editContent(event, contentId) {
    event.stopPropagation();
    const content = currentContents.find((item) => item.module_content_id === contentId);

    if (content) {
        openContentModal(content);
    }
}

async function deleteContent(event, contentId) {
    event.stopPropagation();

    if (!confirm("Delete this module content?")) {
        return;
    }

    await axios.delete(`/api/module-content/${contentId}`);
    loadModuleContent();
}

function showContentStatus(message, type = "info") {
    const statusMessage = document.getElementById("content-status-message");

    if (!statusMessage) {
        return;
    }

    if (statusMessageTimer) {
        clearTimeout(statusMessageTimer);
    }

    statusMessage.textContent = message;
    statusMessage.className = message
        ? `content-status-message ${type} visible`
        : "content-status-message";

    if (message && type !== "info") {
        statusMessageTimer = setTimeout(() => {
            statusMessage.classList.remove("visible");
        }, 5000);
    }
}

async function saveContent() {
    const editingContentId = currentEditingContentId;
    const title = document.getElementById("content-title-input").value.trim();
    const contentType = document.getElementById("content-type-input").value;
    const contentCategory = document.getElementById("content-category-input").value || null;
    const contentFile = document.getElementById("content-file-input").files[0];
    const position = Number(document.getElementById("content-position-input").value || 0);
    const existingContent = currentEditingContentId
        ? currentContents.find((content) => content.module_content_id === currentEditingContentId)
        : null;

    if (!title) {
        showContentStatus("Please enter a content title", "error");
        return;
    }

    if (!Number.isInteger(position) || position < 1) {
        showContentStatus("Please enter a display order of 1 or higher", "error");
        return;
    }

    if (!contentFile && !existingContent?.content_url) {
        showContentStatus("Please choose a file", "error");
        return;
    }

    closeContentModal();
    showContentStatus(contentFile ? "Uploading content..." : "Saving content...");

    try {
        const uploadedContent = contentFile
            ? await uploadModuleContentFile(contentFile, contentType)
            : {
                secure_url: existingContent.content_url,
                public_id: existingContent.cloudinary_public_id,
            };

        showContentStatus("Saving content...");

        const payload = {
            module_id: Number(moduleId),
            content_type: contentType,
            content_category: contentCategory,
            title,
            content_url: uploadedContent.secure_url,
            cloudinary_public_id: uploadedContent.public_id || null,
            position,
        };

        if (editingContentId) {
            await axios.put(`/api/module-content/${editingContentId}`, payload);
        } else {
            await axios.post("/api/module-content", payload);
        }

        await loadModuleContent();
        showContentStatus("Content saved.", "success");
    } catch (error) {
        showContentStatus(error.response?.data || "Failed to save content.", "error");
    }
}

async function uploadModuleContentFile(file, contentType) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", `lms/module-content/${contentType}`);

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data;
}

function bindContentControls() {
    document.getElementById("add-content-btn")?.addEventListener("click", () => openContentModal());
    document.getElementById("save-content-btn")?.addEventListener("click", saveContent);
    document.getElementById("close-content-modal-btn")?.addEventListener("click", closeContentModal);
}

async function initModuleContentPage() {
    bindContentControls();
    await loadManageAccess();
    await loadModuleProgress();
    await loadModuleContent();
}

initModuleContentPage();
