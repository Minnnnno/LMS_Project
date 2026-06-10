document.addEventListener("DOMContentLoaded", () => {
    const currentPath = window.location.pathname.replace(/\/$/, "") || "/";
    const sectionPaths = {
        "/course": "/courses",
        "/module-content": "/courses",
        "/pdf-viewer": "/courses",
    };
    const activePath = sectionPaths[currentPath] || sectionPaths[`/${currentPath.split("/")[1]}`] || currentPath;

    document.querySelectorAll(".sidebar-nav .nav-link").forEach((link) => {
        const linkPath = new URL(link.href, window.location.origin).pathname.replace(/\/$/, "") || "/";
        const isActive = linkPath === activePath;

        link.classList.toggle("active", isActive);

        if (isActive) {
            link.setAttribute("aria-current", "page");
        } else {
            link.removeAttribute("aria-current");
        }
    });
});
