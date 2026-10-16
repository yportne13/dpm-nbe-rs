// The code editor: a textarea over a highlighted mirror.
//
// The textarea keeps native editing (undo, IME, selection, accessibility) while
// a `<pre>` behind it draws the tokens the language server reports and the
// squiggles of its diagnostics.  The two layers share font metrics, padding and
// line height, so a point in the mirror maps to a character offset and a caret
// offset maps back to a screen rectangle.

import { offsetAt, positionAt } from './lsp.js';

const ESCAPES = { '&': '&amp;', '<': '&lt;', '>': '&gt;' };
const escapeHtml = (text) => text.replace(/[&<>]/g, (c) => ESCAPES[c]);

const TOKEN_CLASS = {
  ident: 'tok-ident',
  keyword: 'tok-keyword',
  punct: 'tok-punct',
  comment: 'tok-comment',
  unknown: 'tok-unknown',
};

/** Minimal markdown: fenced code blocks, inline code, emphasis, headings. */
export function renderMarkdown(markdown) {
  const lines = String(markdown).split('\n');
  let html = '';
  let inFence = false;
  for (const line of lines) {
    if (line.trimStart().startsWith('```')) {
      html += inFence ? '</code></pre>' : '<pre class="md-code"><code>';
      inFence = !inFence;
      continue;
    }
    if (inFence) {
      html += `${escapeHtml(line)}\n`;
      continue;
    }
    if (line.trim() === '') {
      html += '<div class="md-gap"></div>';
      continue;
    }
    const inline = escapeHtml(line)
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    html += `<p>${inline}</p>`;
  }
  if (inFence) html += '</code></pre>';
  return html;
}

export class CodeEditor {
  constructor(root, options = {}) {
    this.root = root;
    this.handlers = new Map();
    this.tokens = [];
    this.diagnostics = [];
    this.text = options.text ?? '';
    this.completion = null;
    this.completionIndex = 0;
    this.completionPrefix = '';
    this.completionItems = [];

    root.classList.add('dpm-editor');
    root.innerHTML = `
      <div class="gutter"><div class="gutter-inner"></div></div>
      <div class="code">
        <pre class="overlay" aria-hidden="true"></pre>
        <textarea class="input" wrap="off" spellcheck="false" autocapitalize="off"
                  autocomplete="off" aria-label="dpm-nbe source"></textarea>
      </div>`;
    this.gutter = root.querySelector('.gutter-inner');
    this.overlay = root.querySelector('.overlay');
    this.input = root.querySelector('.input');
    this.input.value = this.text;

    this.tooltip = document.createElement('div');
    this.tooltip.className = 'hover-card';
    this.tooltip.hidden = true;
    root.appendChild(this.tooltip);

    this.popup = document.createElement('div');
    this.popup.className = 'completion-card';
    this.popup.hidden = true;
    root.appendChild(this.popup);

    this.mirror = document.createElement('pre');
    this.mirror.className = 'measure';
    this.mirror.setAttribute('aria-hidden', 'true');
    root.appendChild(this.mirror);

    this.bind();
    this.render();
    this.setCaretOffset(0, { scroll: false });
  }

  on(event, handler) {
    if (!this.handlers.has(event)) this.handlers.set(event, []);
    this.handlers.get(event).push(handler);
    return this;
  }

  emit(event, payload) {
    for (const handler of this.handlers.get(event) ?? []) handler(payload);
  }

  // -- state ---------------------------------------------------------------

  get value() {
    return this.text;
  }

  setValue(text, { caret } = {}) {
    this.text = text;
    this.input.value = text;
    this.render();
    if (caret !== undefined) this.setCaretOffset(caret);
    else this.setCaretOffset(0, { scroll: false });
  }

  caretPosition() {
    return positionAt(this.text, this.input.selectionStart);
  }

  caretOffset() {
    return this.input.selectionStart;
  }

  setCaretOffset(offset, { scroll = true } = {}) {
    const clamped = Math.max(0, Math.min(offset, this.text.length));
    this.input.setSelectionRange(clamped, clamped);
    this.input.focus({ preventScroll: !scroll });
    this.scrollCaretIntoView(clamped);
    this.emit('caret', { position: positionAt(this.text, clamped), offset: clamped });
  }

  /** Select `[start, end)` and reveal it (used by "go to definition"). */
  selectRange(start, end) {
    const from = Math.max(0, Math.min(start, this.text.length));
    const to = Math.max(from, Math.min(end, this.text.length));
    this.input.focus();
    this.input.setSelectionRange(from, to);
    this.scrollCaretIntoView(from);
    this.emit('caret', { position: positionAt(this.text, from), offset: from });
  }

  setTokens(tokens) {
    this.tokens = tokens ?? [];
    this.render();
  }

  setDiagnostics(diagnostics) {
    this.diagnostics = diagnostics ?? [];
    this.render();
  }

  // -- geometry ------------------------------------------------------------

  /** Viewport rectangle of a text offset, measured with a mirror element. */
  rectAtOffset(offset) {
    const position = positionAt(this.text, offset);
    const lines = this.text.split('\n');
    const prefix = (lines[position.line] ?? '').slice(0, position.character);
    this.mirror.textContent = prefix;
    const mirrorRect = this.mirror.getBoundingClientRect();
    const rect = this.overlay.getBoundingClientRect();
    const style = getComputedStyle(this.overlay);
    const padLeft = parseFloat(style.paddingLeft) || 0;
    const padTop = parseFloat(style.paddingTop) || 0;
    const lineHeight = parseFloat(style.lineHeight) || 20;
    return {
      left: rect.left + padLeft + mirrorRect.width - this.input.scrollLeft,
      top: rect.top + padTop + position.line * lineHeight - this.input.scrollTop,
      height: lineHeight,
      line: position.line,
    };
  }

  /** Character offset of a viewport point, via the mirror's hit testing. */
  offsetAtPoint(clientX, clientY) {
    // The mirror ignores pointer events so the textarea stays editable; enable
    // them for the hit test only.
    const previous = this.overlay.style.pointerEvents;
    this.overlay.style.pointerEvents = 'auto';
    let range = null;
    try {
      if (document.caretRangeFromPoint) {
        range = document.caretRangeFromPoint(clientX, clientY);
      } else if (document.caretPositionFromPoint) {
        const position = document.caretPositionFromPoint(clientX, clientY);
        if (position) {
          range = { startContainer: position.offsetNode, startOffset: position.offset };
        }
      }
    } finally {
      this.overlay.style.pointerEvents = previous;
    }
    if (!range || !this.overlay.contains(range.startContainer)) return null;
    return textOffsetWithin(this.overlay, range.startContainer, range.startOffset);
  }

  scrollCaretIntoView(offset) {
    const rect = this.rectAtOffset(offset);
    const codeRect = this.overlay.getBoundingClientRect();
    const margin = rect.height;
    if (rect.top < codeRect.top + margin) {
      this.input.scrollTop -= codeRect.top + margin - rect.top;
    } else if (rect.top > codeRect.bottom - margin) {
      this.input.scrollTop += rect.top - (codeRect.bottom - margin);
    }
    const leftEdge = codeRect.left + 40;
    if (rect.left < leftEdge) {
      this.input.scrollLeft -= leftEdge - rect.left;
    } else if (rect.left > codeRect.right - 40) {
      this.input.scrollLeft += rect.left - (codeRect.right - 40);
    }
  }

  // -- rendering -----------------------------------------------------------

  render() {
    const text = this.text;
    const marks = [];
    for (const token of this.tokens) {
      const start = offsetAt(text, token.range.start);
      const end = offsetAt(text, token.range.end);
      if (end > start) marks.push({ start, end, cls: TOKEN_CLASS[token.type] ?? 'tok-unknown' });
    }
    const squiggles = this.diagnostics.map((diagnostic) => {
      const start = Math.min(offsetAt(text, diagnostic.range.start), text.length);
      const end = Math.max(offsetAt(text, diagnostic.range.end), start + 1);
      return { start, end: Math.min(end, text.length) };
    });

    const bounds = new Set([0, text.length]);
    for (const mark of [...marks, ...squiggles]) {
      if (mark.start >= 0 && mark.start <= text.length) bounds.add(mark.start);
      if (mark.end >= 0 && mark.end <= text.length) bounds.add(mark.end);
    }
    const stops = [...bounds].sort((a, b) => a - b);

    let html = '';
    let markAt = 0;
    let squiggleAt = 0;
    for (let i = 0; i < stops.length - 1; i += 1) {
      const from = stops[i];
      const to = stops[i + 1];
      if (to <= from) continue;
      while (markAt < marks.length && marks[markAt].end <= from) markAt += 1;
      while (squiggleAt < squiggles.length && squiggles[squiggleAt].end <= from) squiggleAt += 1;
      const classes = [];
      const mark = marks[markAt];
      if (mark && mark.start <= from && mark.end >= to) classes.push(mark.cls);
      const squiggle = squiggles[squiggleAt];
      if (squiggle && squiggle.start <= from && squiggle.end >= to) classes.push('squiggle');
      const chunk = escapeHtml(text.slice(from, to));
      html += classes.length ? `<span class="${classes.join(' ')}">${chunk}</span>` : chunk;
    }
    // A trailing newline keeps the last empty line at full height.
    this.overlay.innerHTML = `${html}\n`;

    const lineCount = text.split('\n').length;
    const errorLines = new Set(
      this.diagnostics
        .filter((d) => (d.severity ?? 1) === 1)
        .map((d) => d.range.start.line),
    );
    const warningLines = new Set(
      this.diagnostics
        .filter((d) => (d.severity ?? 1) === 2)
        .map((d) => d.range.start.line),
    );
    let gutter = '';
    for (let line = 0; line < lineCount; line += 1) {
      const cls = errorLines.has(line)
        ? 'ln has-error'
        : warningLines.has(line)
          ? 'ln has-warning'
          : 'ln';
      gutter += `<div class="${cls}">${line + 1}</div>`;
    }
    this.gutter.innerHTML = gutter;
    this.syncScroll();
  }

  syncScroll() {
    this.overlay.scrollTop = this.input.scrollTop;
    this.overlay.scrollLeft = this.input.scrollLeft;
    this.gutter.style.transform = `translateY(${-this.input.scrollTop}px)`;
  }

  // -- hover ---------------------------------------------------------------

  showHover(markdown, rect) {
    this.tooltip.innerHTML = renderMarkdown(markdown);
    this.tooltip.hidden = false;
    const width = this.tooltip.offsetWidth;
    const maxLeft = window.scrollX + document.documentElement.clientWidth - width - 12;
    this.tooltip.style.left = `${Math.max(8, Math.min(rect.left, maxLeft))}px`;
    this.tooltip.style.top = `${rect.top + rect.height + 6}px`;
  }

  hideHover() {
    this.tooltip.hidden = true;
  }

  // -- completion ----------------------------------------------------------

  openCompletion(items, { prefix, onAccept }) {
    this.completionItems = items;
    this.completionPrefix = prefix ?? '';
    this.completionIndex = 0;
    this.completionAccept = onAccept;
    this.filterCompletion();
  }

  filterCompletion() {
    const needle = this.completionPrefix.toLowerCase();
    this.completionShown = this.completionItems
      .filter((item) => item.label.toLowerCase().startsWith(needle))
      .slice(0, 50);
    if (this.completionShown.length === 0) {
      this.closeCompletion();
      return;
    }
    this.completionIndex = Math.min(this.completionIndex, this.completionShown.length - 1);
    this.renderCompletion();
    const rect = this.rectAtOffset(this.input.selectionStart);
    this.popup.hidden = false;
    this.popup.style.left = `${rect.left}px`;
    this.popup.style.top = `${rect.top + rect.height}px`;
  }

  renderCompletion() {
    this.popup.innerHTML = this.completionShown
      .map((item, index) => {
        const active = index === this.completionIndex ? ' active' : '';
        const kind = item.kind === 6 ? 'var' : 'kw';
        const detail = item.detail ? `<span class="detail">${escapeHtml(item.detail)}</span>` : '';
        return `<div class="item${active}" data-index="${index}">
            <span class="kind ${kind}">${kind}</span>
            <span class="label">${escapeHtml(item.label)}</span>${detail}
          </div>`;
      })
      .join('');
    const active = this.popup.querySelector('.item.active');
    if (active) active.scrollIntoView({ block: 'nearest' });
  }

  moveCompletion(delta) {
    if (this.popup.hidden || !this.completionShown?.length) return false;
    const count = this.completionShown.length;
    this.completionIndex = (this.completionIndex + delta + count) % count;
    this.renderCompletion();
    return true;
  }

  acceptCompletion(index = this.completionIndex) {
    const item = this.completionShown?.[index];
    if (!item) return false;
    const caret = this.input.selectionStart;
    const prefixStart = caret - this.completionPrefix.length;
    const next = this.text.slice(0, prefixStart) + item.label + this.text.slice(caret);
    const caretAfter = prefixStart + item.label.length;
    this.closeCompletion();
    this.setValue(next, { caret: caretAfter });
    this.emit('change', { text: this.text });
    return true;
  }

  closeCompletion() {
    this.popup.hidden = true;
    this.completionShown = [];
  }

  // -- events --------------------------------------------------------------

  bind() {
    this.input.addEventListener('input', () => {
      this.text = this.input.value;
      this.render();
      // Keep an open completion list in step with what is being typed.
      if (!this.popup.hidden) {
        const request = this.completionRequest();
        this.completionPrefix = request.prefix;
        this.filterCompletion();
      }
      this.emit('change', { text: this.text, completion: this.completionRequest() });
      this.emit('caret', { position: this.caretPosition(), offset: this.input.selectionStart });
    });

    this.input.addEventListener('scroll', () => {
      this.overlay.scrollTop = this.input.scrollTop;
      this.overlay.scrollLeft = this.input.scrollLeft;
      this.gutter.style.transform = `translateY(${-this.input.scrollTop}px)`;
      this.hideHover();
      this.closeCompletion();
    });

    const caretEvent = () => {
      this.emit('caret', { position: this.caretPosition(), offset: this.input.selectionStart });
    };
    this.input.addEventListener('click', caretEvent);
    this.input.addEventListener('focus', caretEvent);

    this.input.addEventListener('keydown', (event) => {
      if (!this.popup.hidden) {
        if (event.key === 'ArrowDown') {
          event.preventDefault();
          this.moveCompletion(1);
          return;
        }
        if (event.key === 'ArrowUp') {
          event.preventDefault();
          this.moveCompletion(-1);
          return;
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
          event.preventDefault();
          this.acceptCompletion();
          return;
        }
        if (event.key === 'Escape') {
          event.preventDefault();
          this.closeCompletion();
          return;
        }
      }
      if (event.key === 'Tab') {
        event.preventDefault();
        const start = this.input.selectionStart;
        this.setValue(`${this.text.slice(0, start)}  ${this.text.slice(this.input.selectionEnd)}`, {
          caret: start + 2,
        });
        this.emit('change', { text: this.text });
        return;
      }
      if (event.key === 'F12') {
        event.preventDefault();
        this.emit('definition', { position: this.caretPosition(), offset: this.input.selectionStart });
        return;
      }
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        this.emit('run', {});
        return;
      }
      if (event.key === ' ' && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        this.emit('complete', this.completionRequest());
        return;
      }
      if (event.key === 'Escape') this.hideHover();
    });

    this.input.addEventListener('keyup', (event) => {
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
        caretEvent();
      }
    });

    this.input.addEventListener('mousemove', (event) => {
      if (event.ctrlKey || event.metaKey) {
        this.root.classList.add('ctrl');
      } else {
        this.root.classList.remove('ctrl');
      }
    });
    this.input.addEventListener('mouseleave', () => this.root.classList.remove('ctrl'));

    this.input.addEventListener('mousedown', (event) => {
      if (event.ctrlKey || event.metaKey) {
        const offset = this.offsetAtPoint(event.clientX, event.clientY);
        if (offset !== null) {
          event.preventDefault();
          this.setCaretOffset(offset);
          this.emit('definition', { position: positionAt(this.text, offset), offset });
          return;
        }
      }
      this.hideHover();
      this.closeCompletion();
    });

    this.popup.addEventListener('mousedown', (event) => {
      const item = event.target.closest('.item');
      if (!item) return;
      event.preventDefault();
      this.acceptCompletion(Number(item.dataset.index));
    });

    window.addEventListener('resize', () => this.hideHover());
  }

  /** The word being typed at the caret, used to filter completion. */
  completionRequest() {
    const caret = this.input.selectionStart;
    let start = caret;
    while (start > 0 && /[A-Za-z0-9']/.test(this.text[start - 1])) start -= 1;
    return {
      position: positionAt(this.text, caret),
      offset: caret,
      prefix: this.text.slice(start, caret),
      prefixStart: start,
    };
  }
}

/** Character offset of `(node, offset)` inside `root`, counting text nodes. */
function textOffsetWithin(root, node, offset) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let total = 0;
  let current = walker.nextNode();
  while (current) {
    if (current === node) return total + offset;
    total += current.data.length;
    current = walker.nextNode();
  }
  // `node` may be the root element itself, with `offset` a child index.
  if (node === root) {
    let sum = 0;
    for (let i = 0; i < offset && i < root.childNodes.length; i += 1) {
      sum += root.childNodes[i].textContent.length;
    }
    return sum;
  }
  return total;
}
