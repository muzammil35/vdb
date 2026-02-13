// ---- Search + highlight state ----
export const state = {
  currentMatchIndex: 0,
  backendMatches: [],
  pagesReady: 0,
  currentSearchQuery: '',
  lastScrolledMatch: null,
};


// ---- Backend search ----
export async function performSearch(query, pdfViewer, updateMatchCounter) {
  state.currentSearchQuery = query;

  try {
    const response = await fetch(
      `/api/search?q=${encodeURIComponent(query)}`
    );
    if (!response.ok) throw new Error('Search request failed');

    const results = await response.json();
    state.backendMatches = results.slice(0, 5);
    state.currentMatchIndex = 0;
 
    if (state.backendMatches.length === 0) {
      updateMatchCounter(0, 0);
      clearHighlights();
      return;
    }

    highlightCurrentBackendMatch(pdfViewer, updateMatchCounter);
  } catch (err) {
    console.error('Backend search error:', err);
    updateMatchCounter(0, 0);
  }
}

export async function performSearchWithId(query, Id,  pdfViewer, updateMatchCounter) {
  state.currentSearchQuery = query;

  try {
    const response = await fetch(
      `/api/search?q=${encodeURIComponent(query)}&id=${encodeURIComponent(Id)}`
    );
    if (!response.ok) throw new Error('Search request failed');

    const results = await response.json();
    state.backendMatches = results.slice(0, 5);
    state.currentMatchIndex = 0;

    if (state.backendMatches.length === 0) {
      updateMatchCounter(0, 0);
      clearHighlights();
      return;
    }

    highlightCurrentBackendMatch(pdfViewer, updateMatchCounter);
  } catch (err) {
    console.error('Backend search error:', err);
    updateMatchCounter(0, 0);
  }
}

export function highlightCurrentBackendMatch(pdfViewer, updateMatchCounter) {
  if (state.backendMatches.length === 0) return;

  const match = state.backendMatches[state.currentMatchIndex];

  pdfViewer.scrollPageIntoView({ pageNumber: match.page });

  const words = match.text.trim().split(/\s+/);
  const first3 = words.slice(0, 3).join(' ');

  setTimeout(() => {
    highlightMatchManually(match.page, first3);
  }, 200);

  updateMatchCounter(
    state.currentMatchIndex + 1,
    state.backendMatches.length
  );
}

// ---- Highlighting ----

export function highlightMatchManually(pageNumber, searchText) {
  const pageDiv = document.querySelector(
    `[data-page-number="${pageNumber}"]`
  );
  if (!pageDiv) return;

  const textLayer = pageDiv.querySelector('.textLayer');
  if (!textLayer) return;

  clearHighlights(textLayer);

  const spans = Array.from(textLayer.querySelectorAll('span'));

  let fullText = '';
  const spanMap = [];

  spans.forEach((span, index) => {
    const text = span.textContent;
    for (let i = 0; i < text.length; i++) {
      spanMap.push(index);
    }
    fullText += text;
  });

  const idx = fullText
    .toLowerCase()
    .indexOf(searchText.toLowerCase());

  if (idx === -1) return;

  const highlightEnd = Math.min(idx + 500, fullText.length);
  const spansToHighlight = new Set();

  for (let i = idx; i < highlightEnd; i++) {
    spansToHighlight.add(spanMap[i]);
  }

  spansToHighlight.forEach(i => {
    spans[i].classList.add('backend-highlight');
  });

  const first = spans[[...spansToHighlight][0]];
  if (first) {
    setTimeout(() => {
      scrollIntoViewIfNotVisible(first);
    }, 100);
  }
}

export function clearHighlights(root = document) {
  root
    .querySelectorAll('.backend-highlight, .post-match-highlight')
    .forEach(el =>
      el.classList.remove('backend-highlight', 'post-match-highlight')
    );
}

// ---- Scrolling ----

export function scrollIntoViewIfNotVisible(el) {
    el.scrollIntoView({ block: 'center', behavior: 'instant' });
    state.lastScrolledMatch = el;
}
