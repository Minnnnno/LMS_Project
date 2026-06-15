// lms-core.js — Shared OOP library for SkillUp LMS
// Loaded globally from base.html before any page-specific scripts.

// ---------------------------------------------------------------------------
// HtmlUtils — safe HTML escaping used by every page
// ---------------------------------------------------------------------------
window.HtmlUtils = {
    escape(value) {
        return String(value ?? "")
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#39;");
    },
};

// ---------------------------------------------------------------------------
// Course — entity class that owns all price / enrollment display logic
// ---------------------------------------------------------------------------
window.Course = class Course {
    constructor(data) {
        this.id          = data.course_id;
        this.name        = data.name || "Untitled course";
        this.description = data.description || "";
        this.imageUrl    = data.background_image_url || "";
        this.currency    = (data.currency || "SGD").toUpperCase();
        this.status      = data.status || "draft";
        this._raw        = data;
    }

    // Canonical cents value — resolves from either price (decimal) or price_cents
    get priceCents() {
        const fromDecimal = Math.round(Number(this._raw.price) * 100);
        if (Number.isFinite(fromDecimal) && fromDecimal > 0) return fromDecimal;
        const fromCents = Number(this._raw.price_cents);
        if (Number.isFinite(fromCents) && fromCents > 0) return fromCents;
        return null;
    }

    get isPaid() {
        return Boolean(this._raw.is_paid) || (this.priceCents !== null && this.priceCents > 0);
    }

    get formattedPrice() {
        if (!this.isPaid) return "Free";
        if (this.priceCents === null) return "Price unavailable";
        return new Intl.NumberFormat("en-SG", {
            style: "currency",
            currency: this.currency,
        }).format(this.priceCents / 100);
    }

    get enrollLabel() {
        return this.isPaid ? "Buy Course" : "Enroll Now";
    }
};

// ---------------------------------------------------------------------------
// LmsApi — Axios wrapper with centralized 401 redirect and error handling
// ---------------------------------------------------------------------------
window.LmsApi = class LmsApi {
    static async get(url) {
        const response = await axios.get(url);
        return response.data;
    }

    static async post(url, payload) {
        const response = await axios.post(url, payload);
        return response.data;
    }

    static async put(url, payload) {
        const response = await axios.put(url, payload);
        return response.data;
    }

    static async delete(url) {
        const response = await axios.delete(url);
        return response.data;
    }

    // Redirects to /login on 401; re-throws all other errors
    static handleError(error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return null;
        }
        throw error;
    }

    // Like get() but returns null on any error instead of throwing
    static async safeGet(url) {
        try {
            return await LmsApi.get(url);
        } catch (error) {
            if (error.response?.status === 401) {
                window.location.href = "/login";
            }
            return null;
        }
    }
};

// ---------------------------------------------------------------------------
// PageState — renders loading / empty / error / content into a container
// ---------------------------------------------------------------------------
window.PageState = class PageState {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
    }

    loading(message = "Loading...") {
        if (!this.container) return;
        this.container.innerHTML = `
            <div class="d-flex align-items-center gap-2 text-muted py-4 px-2">
                <div class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></div>
                <span>${HtmlUtils.escape(message)}</span>
            </div>`;
    }

    empty(message, icon = "bi-inbox") {
        if (!this.container) return;
        this.container.innerHTML = `
            <div class="text-center text-muted py-5">
                <i class="bi ${icon} fs-2 d-block mb-2" aria-hidden="true"></i>
                <p class="mb-0">${HtmlUtils.escape(message)}</p>
            </div>`;
    }

    error(message = "Something went wrong. Please try again.") {
        if (!this.container) return;
        this.container.innerHTML = `
            <div class="alert alert-danger d-flex align-items-center gap-2 mb-0" role="alert">
                <i class="bi bi-exclamation-triangle-fill flex-shrink-0" aria-hidden="true"></i>
                <span>${HtmlUtils.escape(message)}</span>
            </div>`;
    }

    html(markup) {
        if (!this.container) return;
        this.container.innerHTML = markup;
    }
};
