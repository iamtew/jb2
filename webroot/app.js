// Copy in the click itself: clipboard.writeText() is async and loses the
// user gesture, and it throws if the document is not focused (OBS dock).
async function copyText(text) {
  const el = document.createElement('textarea');
  el.value = text;
  el.setAttribute('readonly', '');
  el.style.cssText = 'position:fixed;left:-9999px';
  document.body.appendChild(el);
  el.select();
  let ok = false;
  try {
    ok = document.execCommand('copy');
  } catch (_) {
    /* fall through */
  }
  el.remove();
  if (ok) return;
  window.focus();
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  throw new Error('copy failed');
}

document.addEventListener('DOMContentLoaded', () => {
  const buttons = document.querySelectorAll('[data-copy]');

  buttons.forEach((button) => {
    button.addEventListener('click', async () => {
      const value = button.getAttribute('data-copy');
      if (!value) return;

      try {
        await copyText(window.location.origin + value);
        const original = button.textContent;
        button.textContent = 'Copied!';
        window.setTimeout(() => {
          button.textContent = original;
        }, 1200);
      } catch (err) {
        button.textContent = 'Copy failed';
      }
    });
  });
});
