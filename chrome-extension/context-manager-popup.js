// Maintain a list of tags for synchronization
let tagsList = [];

document.addEventListener("DOMContentLoaded", async () => {
    const statusElement = document.getElementById("status");
    const tagsInput = document.getElementById("tagsInput");

    // Parse URL parameters for snippet and URL
    const urlParams = new URLSearchParams(window.location.search);
    const snippet = urlParams.get("snippet");
    const tabUrl = urlParams.get("url");

    handleContextManager(tabUrl, snippet, statusElement, tagsInput);
});

function handleContextManager(tabUrl, snippet, statusElement, tagsInput) {
    // Enable the tags input field
    tagsInput.classList.add("enabled");

    // Create and enable the "Send Snippet" button
    const sendSnippetButton = document.createElement("button");
    sendSnippetButton.textContent = "Send Snippet";
    sendSnippetButton.classList.add("enabled", "bg-green-500", "text-white", "px-4", "py-2", "rounded", "hover:bg-green-600", "focus:outline-none", "focus:ring", "focus:ring-green-300");
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
                        statusElement.textContent = "Snippet sent successfully with tags: " + tags;
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

    // Add tag on Enter or comma
    tagsInput.addEventListener("keydown", async function (event) {
        if (event.key === "," || event.key === "Enter") {
            event.preventDefault();

            const input = event.target.value.trim().replace(/,$/, "");
            if (input && input !== "✖") {
                addTag(input);
                tagsList.push(input); // Add the tag to the list
                event.target.value = ""; // Clear the input field
            }
        }
    });
}

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
