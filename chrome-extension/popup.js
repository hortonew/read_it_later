// Maintain a list of tags for synchronization
let tagsList = [];
let isSnippetMode = false; // Track if the current mode is for a snippet

document.addEventListener("DOMContentLoaded", async () => {
    const statusElement = document.getElementById("status");
    const tagsInput = document.getElementById("tagsInput");
    const removeButton = document.getElementById("removeUrlButton");

    // Parse URL parameters for snippet and URL
    const urlParams = new URLSearchParams(window.location.search);
    const snippet = urlParams.get("snippet");
    const tabUrl = urlParams.get("url");

    // Determine the mode (snippet or URL)
    if (snippet && snippet.trim() !== "") {
        isSnippetMode = true; // Enable snippet mode
        handleSnippet(tabUrl, snippet, statusElement, tagsInput);
    } else {
        const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
        handleUrl(tab.url, statusElement, removeButton, tagsInput);
    }
});

function handleSnippet(tabUrl, snippet, statusElement, tagsInput) {
    // Enable the tags input field
    tagsInput.classList.add("enabled");

    // Create and enable the "Send Snippet" button
    const sendSnippetButton = document.createElement("button");
    sendSnippetButton.textContent = "Send Snippet";
    sendSnippetButton.classList.add("enabled");
    document.body.appendChild(sendSnippetButton);

    // Set up the button click handler for sending snippets
    sendSnippetButton.addEventListener("click", async () => {
        const tags = tagsList.join(",");
        console.log("Sending snippet with tags:", tags); // Log the tags being sent

        try {
            chrome.runtime.sendMessage(
                { action: "sendSnippet", url: tabUrl, snippet, tags },
                response => {
                    if (response.status === "success") {
                        statusElement.textContent = "Snippet sent successfully!";
                        chrome.storage.local.remove("snippetData"); // Clear snippet data
                        window.close(); // Close popup after successful submission
                    } else {
                        statusElement.textContent = `Error: ${response.error}`;
                    }
                }
            );
        } catch (error) {
            console.error("Failed to send snippet:", error);
            statusElement.textContent = `Error: ${error.message}`;
        }
    });

    // Allow submitting snippet by hitting Enter
    tagsInput.addEventListener("keydown", async function (event) {
        if (event.key === "Enter") {
            event.preventDefault();
            sendSnippetButton.click();
        }
    });
}

function handleUrl(tabUrl, statusElement, removeButton, tagsInput) {
    // Enable buttons initially
    removeButton.classList.add("enabled");
    tagsInput.classList.add("enabled");

    // Attempt to send the URL
    try {
        chrome.runtime.sendMessage(
            { action: "sendUrl", url: tabUrl },
            response => {
                if (response.status === "success") {
                    statusElement.textContent = "URL sent successfully!";
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to send URL:", error);
        statusElement.textContent = `Error: ${error.message}`;
    }
}

// Remove URL handler
document.getElementById("removeUrlButton").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

    try {
        chrome.runtime.sendMessage(
            { action: "removeUrl", url: tab.url },
            response => {
                const statusElement = document.getElementById("status");
                if (response.status === "success") {
                    statusElement.textContent = "URL removed successfully!";
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to remove URL:", error);
    }
});

// Add tag on Enter or comma
document.getElementById("tagsInput").addEventListener("keydown", async function (event) {
    if (event.key === "," || event.key === "Enter") {
        event.preventDefault();

        const input = event.target.value.trim().replace(/,$/, "");
        if (input && input !== "✖") {
            addTag(input);
            tagsList.push(input); // Add the tag to the list
            event.target.value = ""; // Clear the input field

            if (!isSnippetMode) {
                // Sync tags only in URL mode
                syncTags();
            }
        }
    }
});

// Function to add a tag to the UI
function addTag(text) {
    const tagContainer = document.getElementById("tagContainer");

    // Check for duplicate tags
    if (tagsList.includes(text)) {
        const statusElement = document.getElementById("status");
        statusElement.textContent = "Tag already exists!";
        return;
    }

    // Create a new tag element
    const tag = document.createElement("div");
    tag.className = "tag";

    const span = document.createElement("span");
    span.textContent = text;
    tag.appendChild(span);

    const close = document.createElement("span");
    close.textContent = "✖";
    close.className = "close";
    close.addEventListener("click", function () {
        tagContainer.removeChild(tag);
        removeTag(text);

        if (!isSnippetMode) {
            // Sync tags only in URL mode
            syncTags();
        }
    });

    tag.appendChild(close);
    tagContainer.insertBefore(tag, tagsInput);
}

function removeTag(text) {
    const index = tagsList.indexOf(text);
    if (index > -1) {
        tagsList.splice(index, 1);
    }
}

async function syncTags() {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

    try {
        chrome.runtime.sendMessage(
            { action: "sendTags", url: tab.url, tags: tagsList.join(",") },
            response => {
                const statusElement = document.getElementById("status");
                if (response.status === "success") {
                    statusElement.textContent = "Tags added!";
                } else {
                    statusElement.textContent = `Error: ${response.error}`;
                }
            }
        );
    } catch (error) {
        console.error("Failed to sync tags:", error);
    }
}
