// Config loader tests.
//
// Covers: valid env → typed Config, trailing slash stripping, missing
// required vars throwing, transport selection, invalid port rejection.

import { loadConfig } from "../src/config";

describe("loadConfig", () => {
  const originalEnv = process.env;

  beforeEach(() => {
    process.env = { ...originalEnv };
    delete process.env.CREAM_API_URL;
    delete process.env.CREAM_API_KEY;
    delete process.env.MCP_TRANSPORT;
    delete process.env.MCP_HTTP_PORT;
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  it("loads valid config from environment", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    process.env.CREAM_API_KEY = "test-key";
    const config = loadConfig();
    expect(config.apiBaseUrl).toBe("http://localhost:8080");
    expect(config.apiKey).toBe("test-key");
    expect(config.transport).toBe("stdio");
    expect(config.httpPort).toBe(3002);
  });

  it("strips trailing slash from API URL", () => {
    process.env.CREAM_API_URL = "http://localhost:8080/";
    process.env.CREAM_API_KEY = "test-key";
    const config = loadConfig();
    expect(config.apiBaseUrl).toBe("http://localhost:8080");
  });

  it("throws if CREAM_API_URL is missing", () => {
    process.env.CREAM_API_KEY = "test-key";
    expect(() => loadConfig()).toThrow("CREAM_API_URL is required");
  });

  it("throws if CREAM_API_KEY is missing", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    expect(() => loadConfig()).toThrow("CREAM_API_KEY is required");
  });

  it("selects http transport when MCP_TRANSPORT=http", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    process.env.CREAM_API_KEY = "test-key";
    process.env.MCP_TRANSPORT = "http";
    const config = loadConfig();
    expect(config.transport).toBe("http");
  });

  it("defaults to stdio for any other MCP_TRANSPORT value", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    process.env.CREAM_API_KEY = "test-key";
    process.env.MCP_TRANSPORT = "websocket";
    const config = loadConfig();
    expect(config.transport).toBe("stdio");
  });

  it("throws on invalid port number", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    process.env.CREAM_API_KEY = "test-key";
    process.env.MCP_TRANSPORT = "http";
    process.env.MCP_HTTP_PORT = "not-a-number";
    expect(() => loadConfig()).toThrow(
      "MCP_HTTP_PORT must be a valid port number",
    );
  });

  it("throws on out-of-range port number", () => {
    process.env.CREAM_API_URL = "http://localhost:8080";
    process.env.CREAM_API_KEY = "test-key";
    process.env.MCP_HTTP_PORT = "99999";
    expect(() => loadConfig()).toThrow(
      "MCP_HTTP_PORT must be a valid port number",
    );
  });
});
