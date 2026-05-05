function escapeSvgText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function hashSeed(seed: string): number {
  return Array.from(seed).reduce((hash, character) => {
    return (hash * 31 + character.charCodeAt(0)) >>> 0;
  }, 7);
}

export function buildArtworkDataUrl(seed: string, title: string, subtitle = ""): string {
  const hash = hashSeed(seed || title || "jukeboy");
  const hueA = hash % 360;
  const hueB = (hash * 7) % 360;
  const hueC = (hash * 17) % 360;
  const safeTitle = escapeSvgText(title || "Unknown cartridge");
  const safeSubtitle = escapeSvgText(subtitle || "Jukeboy Companion");
  const initial = escapeSvgText((title.trim().charAt(0) || "J").toUpperCase());

  const svg = `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 900" role="img" aria-label="${safeTitle}">
      <defs>
        <linearGradient id="bg" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0%" stop-color="hsl(${hueA}, 78%, 58%)" />
          <stop offset="55%" stop-color="hsl(${hueB}, 68%, 54%)" />
          <stop offset="100%" stop-color="hsl(${hueC}, 72%, 48%)" />
        </linearGradient>
      </defs>
      <rect width="900" height="900" fill="url(#bg)" rx="76" />
      <g opacity="0.22">
        <circle cx="185" cy="190" r="170" fill="#ffffff" />
        <circle cx="735" cy="165" r="220" fill="#000000" />
        <rect x="130" y="490" width="610" height="210" rx="105" fill="#ffffff" />
      </g>
      <g opacity="0.28">
        <path d="M0 650c90-110 226-165 408-165s333 59 492 175v240H0z" fill="#000000" />
      </g>
      <text x="86" y="256" fill="#ffffff" font-family="Space Grotesk, sans-serif" font-size="240" font-weight="800" letter-spacing="-18">${initial}</text>
      <text x="86" y="744" fill="#ffffff" font-family="Space Grotesk, sans-serif" font-size="72" font-weight="700">${safeTitle}</text>
      <text x="86" y="812" fill="rgba(255,255,255,0.76)" font-family="Space Grotesk, sans-serif" font-size="34" font-weight="500">${safeSubtitle}</text>
    </svg>
  `;

  return `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(svg)}`;
}