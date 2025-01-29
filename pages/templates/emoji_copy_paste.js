document.addEventListener('copy', function(e) {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;

    // Check if the selection includes any .emoji SVGs
    let hasEmojiSvg = false;
    for (let i = 0; i < selection.rangeCount; i++) {
        const range = selection.getRangeAt(i);
        const clonedContent = range.cloneContents();
        if (clonedContent.querySelector('svg.emoji')) {
            hasEmojiSvg = true;
            break;
        }
    }

    if (!hasEmojiSvg) return; // No action needed if no relevant SVGs

    // Process the selection to replace SVGs with alt text
    const tempDiv = document.createElement('div');
    for (let i = 0; i < selection.rangeCount; i++) {
        const range = selection.getRangeAt(i);
        tempDiv.appendChild(range.cloneContents());
    }

    // Replace each SVG with its alt text
    tempDiv.querySelectorAll('svg.emoji').forEach(svg => {
        const altText = svg.getAttribute('alt') || '';
        svg.replaceWith(altText);
    });

    // Update clipboard data
    e.clipboardData.setData('text/html', tempDiv.innerHTML);
    e.clipboardData.setData('text/plain', tempDiv.textContent);
    e.preventDefault();
});
