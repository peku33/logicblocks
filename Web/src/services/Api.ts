export function urlBuild(endpoint: string): string {
  // return `http://${document.location.hostname}:8080${endpoint}`;
  return `http://10.0.2.105:8080${endpoint}`;
}

export async function getJson(endpoint: string): Promise<any> {
  const response = await fetch(urlBuild(endpoint));
  const json = await response.json();
  return json;
}

export async function postJsonEmpty(endpoint: string, data: any): Promise<void> {
  const request = JSON.stringify(data);
  const response = await fetch(
    urlBuild(endpoint), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: request,
  },
  );
  const text = await response.text();
  if (text !== "") {
    throw new Error("Expected empty string");
  }
}
