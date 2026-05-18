export interface Header {
  name: string;
  value: string;
}

export interface Part {
  name?: string;
  fileName?: string;
  contentType?: string;
  bodyStart: number;
  bodyEnd: number;
  headers: Header[];
}

export function boundaryFromContentType(contentType: string): string | undefined;

export function parse(body: Uint8Array, boundary: string): Part[];

export function parseContentType(body: Uint8Array, contentType: string): Part[];
