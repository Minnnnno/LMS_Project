// enrollment.js — Shared enrollment action helpers (depends on lms-core.js)

window.EnrollmentHelper = {
    async enrollFree(courseId) {
        await LmsApi.post(`/api/courses/${courseId}/enroll`);
    },

    async startCheckout(courseId) {
        const data = await LmsApi.post(`/api/courses/${courseId}/checkout`);
        window.location.href = data.checkout_url;
    },

    async getStatus(courseId) {
        return LmsApi.safeGet(`/api/courses/${courseId}/enrollment-status`);
    },
};
