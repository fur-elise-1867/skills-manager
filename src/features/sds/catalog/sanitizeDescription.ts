const allowedTags = new Set(["a", "blockquote", "br", "code", "em", "h1", "h2", "h3", "h4", "li", "ol", "p", "pre", "strong", "ul"]);

function safeHref(value: string | null) {
  if (!value) return null;
  try {
    const url = new URL(value, window.location.origin);
    return ["http:", "https:"].includes(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
}

/**
 * Store descriptions are author-provided rich text. Keep the useful formatting
 * but reduce it to a small inert HTML subset before React receives it.
 */
export function sanitizeDescription(value: string) {
  const document = new DOMParser().parseFromString(value, "text/html");
  for (const element of Array.from(document.body.querySelectorAll("*"))) {
    if (!allowedTags.has(element.tagName.toLowerCase())) {
      if (["script", "style", "template"].includes(element.tagName.toLowerCase())) {
        element.remove();
        continue;
      }
      element.replaceWith(...Array.from(element.childNodes));
      continue;
    }
    const originalHref = element.getAttribute("href");
    for (const attribute of Array.from(element.attributes)) {
      element.removeAttribute(attribute.name);
    }
    if (element.tagName.toLowerCase() === "a") {
      const sanitizedHref = safeHref(originalHref);
      if (sanitizedHref) {
        element.setAttribute("href", sanitizedHref);
        element.setAttribute("target", "_blank");
        element.setAttribute("rel", "noreferrer");
      } else {
        element.replaceWith(...Array.from(element.childNodes));
      }
    }
  }
  return document.body.innerHTML;
}
