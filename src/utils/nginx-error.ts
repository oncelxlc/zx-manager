const configurationErrorCodes = new Set([
  "NGINX_CONFIG_ENCODING_INVALID",
  "NGINX_CONFIG_FILE_TOO_LARGE",
  "NGINX_CONFIG_INCLUDE_UNAUTHORIZED",
]);

export function nginxErrorTranslationKey(code: string | undefined) {
  const normalized = code ?? "NGINX_UNKNOWN";
  return configurationErrorCodes.has(normalized)
    ? `configuration.diagnostics.${normalized}`
    : `errors.${normalized}`;
}
