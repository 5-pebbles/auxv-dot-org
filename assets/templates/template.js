const overlay = document.getElementById('overlay');
const exitButton = document.getElementById('overlay-exit');
const searchInput = document.getElementById('overlay-search-input');
const resultsNav = document.getElementById('overlay-results');

// Track currently selected result
let selectedIndex = -1;

function showOverlay() {
  overlay.style.display = 'block';
  searchInput.focus();
  selectedIndex = -1;
}

function hideOverlay() {
  overlay.style.display = 'none';
  searchInput.value = '';
  selectedIndex = -1;
}

// Start with the Overlay hidden
hideOverlay();

function handleKeyNavigation(e) {
  const results = resultsNav.querySelectorAll('a');
  if (!results.length) return;

  // Handle tab navigation
  if (e.key === 'Tab') {
    e.preventDefault();
    
    if (e.shiftKey) {
      // Shift+Tab: Move up
      selectedIndex = selectedIndex <= 0 ? results.length - 1 : selectedIndex - 1;
    } else {
      // Tab: Move down
      selectedIndex = selectedIndex >= results.length - 1 ? 0 : selectedIndex + 1;
    }
    
    results[selectedIndex].focus();
    updateSelectionStyles(results);
  }
  
  // Handle enter to navigate
  if (e.key === 'Enter' && selectedIndex !== -1) {
    results[selectedIndex].click();
  }
}

function updateSelectionStyles(results) {
  results.forEach((result, index) => {
    result.classList.toggle('selected', index === selectedIndex);
  });
}

// Handle keyboard events
document.addEventListener('keydown', (e) => {
  if (e.key === 's' && 
      !e.ctrlKey && 
      !e.metaKey && 
      document.activeElement.tagName !== 'INPUT' && 
      document.activeElement.tagName !== 'TEXTAREA') {
    e.preventDefault();
    showOverlay();
  }
  
  if (e.key === 'Escape') {
    hideOverlay();
  }
  
  if (overlay.style.display === 'block') {
    handleKeyNavigation(e);
  }
});

function debounce(func, wait) {
  let timeout;
  return function (...args) {
    clearTimeout(timeout);
    timeout = setTimeout(() => func.apply(this, args), wait);
  };
}

function createResultHTML(result) {
  return `
    <a href="/${result.url}" tabindex="0">
      <div class="overlay-result-main">
        <span class="overlay-result-title">${result.title}</span>
        <span class="overlay-result-preview">${result.short}</span>
      </div>
      <span class="overlay-result-path">/${result.url}</span>
    </a>
  `;
}

async function updateSearch(query) {
  try {
    if (!query.trim()) {
      resultsNav.innerHTML = '';
      return;
    }

    const response = await fetch(`/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) throw new Error('Search failed');
    
    const results = await response.json();
    resultsNav.innerHTML = results.map(createResultHTML).join('');
    selectedIndex = -1;
    
  } catch (error) {
    console.error('Search error:', error);
    resultsNav.innerHTML = `
      <div style="padding: 1rem; color: var(--love);">
        Search failed. Please try again.
      </div>
    `;
  }
}

const debouncedSearch = debounce(updateSearch, 300);

searchInput.addEventListener('input', (e) => {
  debouncedSearch(e.target.value);
});
