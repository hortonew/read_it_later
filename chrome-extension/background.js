chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === "sendUrl") {
        fetch("http://localhost:8080/urls/url", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({ url: message.url })
        })
            .then(response => {
                if (response.ok) {
                    sendResponse({ status: "success" });
                } else {
                    sendResponse({ status: "error", error: response.statusText });
                }
            })
            .catch(error => {
                sendResponse({ status: "error", error: error.message });
            });

        return true;

    } else if (message.action === "removeUrl") {
        fetch("http://localhost:8080/urls/delete/by-url", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({ url: message.url })
        })
            .then(response => {
                if (response.ok) {
                    sendResponse({ status: "success" });
                } else {
                    sendResponse({ status: "error", error: response.statusText });
                }
            })
            .catch(error => {
                sendResponse({ status: "error", error: error.message });
            });

        return true;

    } else if (message.action === "sendTags") {
        fetch("http://localhost:8080/urls/tags", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({ url: message.url, tags: message.tags })
        })
            .then(response => {
                if (response.ok) {
                    sendResponse({ status: "success" });
                } else {
                    sendResponse({ status: "error", error: response.statusText });
                }
            })
            .catch(error => {
                sendResponse({ status: "error", error: error.message });
            });

        return true;

    } else if (message.action === "sendTag") {
        fetch("http://localhost:8080/urls/tags", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({ url: message.url, tags: message.tag })
        })
            .then(response => {
                if (response.ok) {
                    sendResponse({ status: "success" });
                } else {
                    sendResponse({ status: "error", error: response.statusText });
                }
            })
            .catch(error => {
                sendResponse({ status: "error", error: error.message });
            });

        return true;
    }
});
