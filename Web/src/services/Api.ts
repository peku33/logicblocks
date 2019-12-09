export function urlBuild(endpoint: string): string {
  return `http://${document.location.hostname}:8080${endpoint}`;
}

export async function get(endpoint: string): Promise<any> {
  const response = await fetch(urlBuild(endpoint));
  const json = await response.json();
  return json;
}
