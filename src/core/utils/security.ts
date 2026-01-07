import { resolve, normalize, isAbsolute } from "node:path";

export function validatePathSecurity(
  filePath: string,
  baseDir: string
): { valid: boolean; normalizedPath: string; error?: string } {
  // Normalize both paths
  const normalizedBase = normalize(resolve(baseDir));
  const normalizedPath = normalize(
    isAbsolute(filePath) ? filePath : resolve(baseDir, filePath)
  );

  // Check that the resolved path is within the base directory
  if (!normalizedPath.startsWith(normalizedBase + "/") && normalizedPath !== normalizedBase) {
    return {
      valid: false,
      normalizedPath,
      error: `Path traversal detected: "${filePath}" resolves outside the allowed directory`,
    };
  }

  return { valid: true, normalizedPath };
}

export function containsDangerousPatterns(filePath: string): boolean {
  // Check for common dangerous patterns
  const dangerousPatterns = [
    /\.\./,        // Parent directory traversal
    /^~\//,         // Home directory (could leak info)
    /^\/etc\//,     // System config
    /^\/var\//,     // System var
    /^\/proc\//,    // Process info
    /^\/sys\//,     // System info
  ];

  return dangerousPatterns.some((pattern) => pattern.test(filePath));
}
