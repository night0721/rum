// Theme Toggle
const themeToggle = document.getElementById('theme-toggle');
const html = document.documentElement;

// Load saved theme
const savedTheme = localStorage.getItem('theme') || 'light';
html.setAttribute('data-theme', savedTheme);
updateThemeIcon(savedTheme);

themeToggle.addEventListener('click', () => {
    const currentTheme = html.getAttribute('data-theme');
    const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', newTheme);
    localStorage.setItem('theme', newTheme);
    updateThemeIcon(newTheme);
});

function updateThemeIcon(theme) {
    const icon = themeToggle.querySelector('.theme-icon');
    if (icon) {
        icon.textContent = theme === 'dark' ? '☀️' : '🌙';
    }
}

// Search Functionality
let searchIndex = [];
let fuse = null;

const searchToggle = document.getElementById('search-toggle');
const searchOverlay = document.getElementById('search-overlay');
const searchInput = document.getElementById('search-input');
const searchResults = document.getElementById('search-results');
const searchClose = document.getElementById('search-close');

// Load search index
fetch('/assets/search-index.json')
    .then(response => response.json())
    .then(data => {
        searchIndex = data;
        if (typeof Fuse !== 'undefined') {
            fuse = new Fuse(data, {
                keys: ['title', 'content'],
                threshold: 0.3,
                includeScore: true,
            });
        }
    })
    .catch(err => console.error('Failed to load search index:', err));

searchToggle.addEventListener('click', () => {
    searchOverlay.classList.add('active');
    searchInput.focus();
});

searchClose.addEventListener('click', () => {
    searchOverlay.classList.remove('active');
    searchInput.value = '';
    searchResults.innerHTML = '';
});

searchOverlay.addEventListener('click', (e) => {
    if (e.target === searchOverlay) {
        searchOverlay.classList.remove('active');
        searchInput.value = '';
        searchResults.innerHTML = '';
    }
});

searchInput.addEventListener('input', (e) => {
    const query = e.target.value.trim();
    
    if (!query) {
        searchResults.innerHTML = '';
        return;
    }
    
    if (!fuse) {
        // Fallback simple search
        const results = searchIndex.filter(item => {
            const title = item.title.toLowerCase();
            const content = item.content.toLowerCase();
            return title.includes(query.toLowerCase()) || content.includes(query.toLowerCase());
        }).slice(0, 10);
        
        displayResults(results, query);
    } else {
        const results = fuse.search(query).slice(0, 10);
        displayResults(results.map(r => r.item), query);
    }
});

function displayResults(results, query) {
    if (results.length === 0) {
        searchResults.innerHTML = '<div class="search-result-item"><p>No results found</p></div>';
        return;
    }
    
    const html = results.map(item => {
        const title = highlightText(item.title, query);
        const snippet = getSnippet(item.content, query);
        const path = item.path || '';
        const version = item.version ? `/${item.version}` : '';
        
        return `
            <div class="search-result-item" onclick="window.location.href='${version}/${path.replace(/\.md$/, '.html')}'">
                <h4>${title}</h4>
                <p>${snippet}</p>
            </div>
        `;
    }).join('');
    
    searchResults.innerHTML = html;
}

function highlightText(text, query) {
    const regex = new RegExp(`(${query})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
}

function getSnippet(content, query, length = 150) {
    const index = content.toLowerCase().indexOf(query.toLowerCase());
    if (index === -1) {
        return content.substring(0, length) + '...';
    }
    
    const start = Math.max(0, index - 50);
    const end = Math.min(content.length, index + query.length + 100);
    let snippet = content.substring(start, end);
    
    if (start > 0) snippet = '...' + snippet;
    if (end < content.length) snippet = snippet + '...';
    
    return highlightText(snippet, query);
}

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    // '/' to open search
    if (e.key === '/' && e.target.tagName !== 'INPUT' && e.target.tagName !== 'TEXTAREA') {
        e.preventDefault();
        searchToggle.click();
    }
    
    // Escape to close search
    if (e.key === 'Escape' && searchOverlay.classList.contains('active')) {
        searchClose.click();
    }
});

// Version switching
function switchVersion(version) {
    const currentPath = window.location.pathname;
    const newPath = currentPath.replace(/^\/([^\/]+)/, `/${version}`);
    window.location.href = newPath;
}

// Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });
});
