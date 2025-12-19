interface Rpc {
  request(
    service: string,
    method: string,
    data: Uint8Array
  ): Promise<Uint8Array>;
}

export class GrpcWebRpc implements Rpc {
  private host: string;

  constructor(host: string) {
    this.host = host;
  }

  async request(
    service: string,
    method: string,
    data: Uint8Array
  ): Promise<Uint8Array> {
    const url = `${this.host}/${service}/${method}`;

    // Frame the request: [0x00, len_be_32, payload]
    const buf = new Uint8Array(5 + data.length);
    buf[0] = 0; // Not compressed
    const len = data.length;
    buf[1] = (len >> 24) & 0xff;
    buf[2] = (len >> 16) & 0xff;
    buf[3] = (len >> 8) & 0xff;
    buf[4] = len & 0xff;
    buf.set(data, 5);

    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/grpc-web+proto",
        "x-grpc-web": "1",
      },
      body: buf,
    });

    if (!response.ok) {
      throw new Error(`RPC Error: ${response.statusText}`);
    }

    const responseBuffer = new Uint8Array(await response.arrayBuffer());

    // Unframe the response
    // Basic implementation: assumes single frame response for unary call
    // Real implementation should handle stream, trailers, etc.
    // Frame format: [flag, len(4bytes), payload]

    if (responseBuffer.length < 5) {
      if (
        response.headers.has("grpc-status") &&
        response.headers.get("grpc-status") !== "0"
      ) {
        throw new Error(
          `gRPC Error Status: ${response.headers.get(
            "grpc-status"
          )} Message: ${response.headers.get("grpc-message")}`
        );
      }
      throw new Error(
        `Invalid response: too short (len=${responseBuffer.length}). Status: ${response.status}`
      );
    }

    // Check flag (buf[0]). If 0x80 bit set, it's trailers.
    // For now assuming success response in first frame.

    const msgLen =
      (responseBuffer[1] << 24) |
      (responseBuffer[2] << 16) |
      (responseBuffer[3] << 8) |
      responseBuffer[4];

    if (responseBuffer.length < 5 + msgLen) {
      throw new Error("Invalid response: incomplete frame");
    }

    return responseBuffer.slice(5, 5 + msgLen);
  }
}
