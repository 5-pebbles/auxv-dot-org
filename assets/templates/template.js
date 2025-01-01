const overlay = document.getElementById('overlay');
const exitButton = document.getElementById('overlay-exit');
const searchInput = document.getElementById('overlay-search-input');
const resultsNav = document.getElementById('overlay-results');


// Functions to toggle overlay:
function showOverlay() {
  overlay.style.display = 'block';
  searchInput.focus(); // Auto-focus the search input
}

function hideOverlay() {
  overlay.style.display = 'none';
  searchInput.value = ''; // Clear the search input when closing
}

// Start with the Overlay hidden:
hideOverlay();


// Handle keyboard events:
document.addEventListener('keydown', (e) => {
  // Open on `s`:
  if (e.key === 's' && 
      !e.ctrlKey && 
      !e.metaKey && 
      document.activeElement.tagName !== 'INPUT' && 
      document.activeElement.tagName !== 'TEXTAREA') {
    e.preventDefault(); // Prevent this 's' from being typed.
    showOverlay();
  }
  
  // Close on `escape`:
  if (e.key === 'Escape') {
    hideOverlay();
  }
});

// Debounce function to limit API calls while typing:
function debounce(func, wait) {
  let timeout;
  return function (...args) {
    clearTimeout(timeout);
    timeout = setTimeout(() => func.apply(this, args), wait);
  };
}

// Create result item HTML:
function createResultHTML(result) {
  return `
    <a href="/${result.url}">
      <div class="overlay-result-main">
        <span class="overlay-result-title">${result.title}</span>
        <span class="overlay-result-preview">${result.short}</span>
      </div>
      <span class="overlay-result-path">/${result.url}</span>
    </a>
  `;
}

// Update search results:
async function updateSearch(query) {
  try {
    if (!query.trim()) {
      resultsNav.innerHTML = ''; // Clear results if query is empty.
      return;
    }

    const response = await fetch(`/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) throw new Error('Search failed');
    
    const results = await response.json();
    resultsNav.innerHTML = results
      .map(createResultHTML)
      .join('');
    
  } catch (error) {
    console.error('Search error:', error);
    resultsNav.innerHTML = `
      <div style="padding: 1rem; color: var(--love);">
        Search failed. Please try again.
      </div>
    `;
  }
}

// Debounced search function to avoid too many API calls:
const debouncedSearch = debounce(updateSearch, 300);

// Add search event listeners:
searchInput.addEventListener('input', (e) => {
  debouncedSearch(e.target.value);
});
