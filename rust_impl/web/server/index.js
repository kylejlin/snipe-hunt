/**
 * Cloudflare Worker entrypoint used by OpenAI Sites.
 *
 * The game is client-side only; this worker delegates files to the static
 * asset binding and sends navigation requests to the React entrypoint.
 */
export default {
  async fetch(request, env) {
    const response = await env.ASSETS.fetch(request);
    if (response.status !== 404 || request.method !== "GET") {
      return response;
    }

    const url = new URL(request.url);
    if (url.pathname.split("/").pop()?.includes(".")) {
      return response;
    }

    url.pathname = "/index.html";
    return env.ASSETS.fetch(new Request(url, request));
  },
};
