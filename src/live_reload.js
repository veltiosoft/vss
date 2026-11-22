(() => {
    const script = document.currentScript;
    const events = new EventSource(new URL("events", script.src));

    events.addEventListener("shutdown", () => events.close());

    events.addEventListener("message", ({ data }) => {
        if (data !== script.dataset.vssRevision) {
            events.close();
            window.location.reload();
        }
    });
})();
