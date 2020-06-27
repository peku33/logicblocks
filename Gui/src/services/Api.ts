export function urlBuild(endpoint: string): string {
  return `/api${endpoint}`;
}

export async function getJson<R>(endpoint: string): Promise<R> {
  const response = await fetch(urlBuild(endpoint));
  const json = await response.json();
  return json;
}

export async function postJsonEmpty<D>(endpoint: string, data: D): Promise<void> {
  const request = JSON.stringify(data);
  const response = await fetch(urlBuild(endpoint), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: request,
  });
  const text = await response.text();
  if (text !== "") {
    throw new Error("Expected empty string");
  }
}
