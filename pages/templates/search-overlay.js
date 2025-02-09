const elements = {
  overlay: document.getElementById('search-overlay'),
  search: document.getElementById('search-input'),
  results: document.getElementById('search-results')
};

// Event listeners:
const toggleOverlay = (show) => {
  elements.overlay.style.display = show ? 'block' : 'none';
  if (show) {
    elements.search.value = '';
    elements.search.focus();
  }
  selectedIndex = -1;
};

// Track currently selected result:
let selectedIndex = -1;
const handleKeyNavigation = (e) => {
  const results = elements.results.querySelectorAll('a');
  if (!results.length) return;

  const { key, shiftKey } = e;
  const isUpKey = key === 'ArrowUp' || (key === 'Tab' && shiftKey);
  const isDownKey = key === 'ArrowDown' || (key === 'Tab' && !shiftKey);

  if (isUpKey) {
    selectedIndex = selectedIndex <= 0 ? results.length - 1 : selectedIndex - 1
  }

  if (isDownKey) {
    selectedIndex = selectedIndex >= results.length - 1 ? 0 : selectedIndex + 1
  }

  if (isUpKey || isDownKey) {
    e.preventDefault();
    results.forEach((result, i) => result.classList.toggle('selected', i === selectedIndex));
  }

  if (e.key === 'Enter' && selectedIndex !== -1) {
    results[selectedIndex].click();
  }
};

document.addEventListener('keydown', (e) => {
  if (e.key === 's' &&
      !e.ctrlKey &&
      !e.metaKey &&
      !['INPUT', 'TEXTAREA'].includes(document.activeElement.tagName)) {
    e.preventDefault();
    toggleOverlay(true);
  }

  if (e.key === 'Escape') toggleOverlay(false);
  if (elements.overlay.style.display === 'block') handleKeyNavigation(e);
});

// Search functionality:
const debounce = (fn, delay) => {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), delay);
  };
};

const createResultHTML = ({ path, title, matched }) => `
  <a href="/${path}" tabindex="0">
    <div class="search-result-container">
      <span class="search-result-title">${title}</span>
      <span class="search-result-preview">${matched}</span>
    </div>
    <div class="search-result-path-container">
      <span class="search-result-path">${path}/</span>
    </div>
  </a>
`;

const updateSearch = async (query) => {
  try {
    if (!query.trim()) {
      elements.results.innerHTML = '';
      return;
    }

    const response = await fetch(`/search?query=${encodeURIComponent(query)}`);
    if (!response.ok) throw new Error('Search failed');

    const results = await response.json();
    elements.results.innerHTML = results.map(createResultHTML).join('');
    selectedIndex = -1;
  } catch (error) {
    console.error('Search error:', error);
    elements.results.innerHTML = '<div style="padding: 1rem; color: var(--love);">Search failed. Please try again.</div>';
  }
};

elements.search.addEventListener('input',
  debounce(e => updateSearch(e.target.value), 500)
);
