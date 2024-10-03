class ApiRequestError extends Error {
  public constructor(response: Response) {
    // eslint-disable-next-line @typescript-eslint/no-base-to-string
    super(response.toString());
  }
}

const API_HOST = (import.meta.env.VITE_API_HOST as string) || "";
export function urlBuild(endpoint: string): string {
  return `${API_HOST}/api${endpoint}`;
}

export async function getJson<R>(endpoint: string): Promise<R> {
  const response = await fetch(urlBuild(endpoint));
  if (!response.ok) {
    throw new ApiRequestError(response);
  }

  const json = (await response.json()) as R;

  return json;
}

export async function postEmpty(endpoint: string): Promise<void> {
  const response = await fetch(urlBuild(endpoint), {
    method: "POST",
  });
  if (!response.ok) {
    throw new ApiRequestError(response);
  }
  const text = await response.text();
  if (text !== "") {
    throw new Error("expected empty string");
  }
}

// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
export async function postJsonEmpty<D>(endpoint: string, data: D): Promise<void> {
  const request = JSON.stringify(data);
  const response = await fetch(urlBuild(endpoint), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: request,
  });
  if (!response.ok) {
    throw new ApiRequestError(response);
  }
  const text = await response.text();
  if (text !== "") {
    throw new Error("expected empty string");
  }
}
