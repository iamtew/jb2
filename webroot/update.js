const LATEST_RELEASE = 'https://github.com/iamtew/jb2/releases/latest';

async function fetchUpdateStatus() {
  try {
    const response = await fetch('/api/update');
    const payload = await response.json();
    return payload.data || {};
  } catch (_) {
    return { updateAvailable: false };
  }
}

function applyUpdateLink(el, prominent) {
  if (!el) return;
  fetchUpdateStatus().then((data) => {
    if (!data.updateAvailable) return;
    el.textContent = 'Update available';
    el.href = data.url || LATEST_RELEASE;
    el.hidden = false;
    if (prominent) el.classList.remove('secondary');
  });
}

document.addEventListener('DOMContentLoaded', () => {
  applyUpdateLink(document.getElementById('downloadLink'), true);
  applyUpdateLink(document.getElementById('aboutUpdate'), false);
});
