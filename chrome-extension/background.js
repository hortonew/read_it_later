// Listen for messages from the popup or other parts of the extension
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

        return true; // Keep the message channel open for async response
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
    } else if (message.action === "sendSnippet") {
        fetch("http://localhost:8080/snippets", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                url: message.url,
                snippet: message.snippet,
                tags: message.tags
            })
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

// Create a context menu item
chrome.runtime.onInstalled.addListener(() => {
    chrome.contextMenus.create({
        id: "sendSnippet",
        title: "Send Snippet to Read It Later",
        contexts: ["selection"] // Show only when text is selected
    });
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener((info, tab) => {
    if (info.menuItemId === "sendSnippet" && info.selectionText) {
        const highlightedText = info.selectionText; // Get selected text
        const tabUrl = tab.url;

        // Add logic to send the snippet
        fetch("http://localhost:8080/snippets", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                url: tabUrl,
                snippet: highlightedText,
                tags: "" // You can enhance this to include tags if needed
            })
        })
            .then(response => {
                if (response.ok) {
                    console.log("Snippet sent successfully!");
                } else {
                    console.error("Error sending snippet:", response.statusText);
                }
            })
            .catch(error => {
                console.error("Failed to send snippet:", error);
            });
    }
});
