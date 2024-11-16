chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === "sendUrl") {
        fetch("http://192.168.1.56:8080/urls/url", {
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

        return true; // Indicates that the response is asynchronous
    }
});
