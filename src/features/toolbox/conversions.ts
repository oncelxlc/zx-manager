export type TimeZone = "utc" | "local";
export type TimeUnit = "seconds" | "milliseconds";

export function formatJson(input: string) {
  JSON.parse(input);
  // Indent the validated tokens directly to preserve large integers and duplicate keys.
  const tokens = input.match(/"(?:\\[\s\S]|[^"\\])*"|[^\s{}\[\],:"]+|[{}\[\],:]/g) ?? [];
  const parts: string[] = [];
  let depth = 0;
  for (let index = 0; index < tokens.length; index++) {
    const token = tokens[index];
    if (token === "{" || token === "[") {
      parts.push(token);
      depth++;
      if (tokens[index + 1] !== "}" && tokens[index + 1] !== "]") parts.push("\n", "  ".repeat(depth));
    } else if (token === "}" || token === "]") {
      depth--;
      if (tokens[index - 1] !== "{" && tokens[index - 1] !== "[") parts.push("\n", "  ".repeat(depth));
      parts.push(token);
    } else if (token === ",") {
      parts.push(",\n", "  ".repeat(depth));
    } else {
      parts.push(token === ":" ? ": " : token);
    }
  }
  return parts.join("");
}

export function convertBase64(input: string, decode: boolean) {
  if (decode) {
    const binary = atob(input);
    const bytes = Uint8Array.from(binary, character => character.charCodeAt(0));
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  }
  const bytes = new TextEncoder().encode(input);
  const parts: string[] = [];
  for (let start = 0; start < bytes.length; start += 8192) {
    parts.push(String.fromCharCode(...bytes.subarray(start, start + 8192)));
  }
  return btoa(parts.join(""));
}

function dateText(date: Date, zone: TimeZone) {
  if (!Number.isFinite(date.getTime())) throw new Error("时间超出可转换范围。");
  const utc = zone === "utc";
  const year = utc ? date.getUTCFullYear() : date.getFullYear();
  if (year < 0 || year > 9999) throw new Error("支持的年份为 0000 至 9999。");
  const month = utc ? date.getUTCMonth() : date.getMonth();
  const day = utc ? date.getUTCDate() : date.getDate();
  const clock = utc ? [date.getUTCHours(), date.getUTCMinutes(), date.getUTCSeconds()]
    : [date.getHours(), date.getMinutes(), date.getSeconds()];
  const ms = utc ? date.getUTCMilliseconds() : date.getMilliseconds();
  const pad = (n: number) => String(n).padStart(2, "0");
  return String(year).padStart(4, "0") + "-" + pad(month + 1) + "-" + pad(day) + " " + clock.map(pad).join(":")
    + (ms ? "." + String(ms).padStart(3, "0") : "");
}

export function convertTimestamp(input: string, reverse: boolean, unit: TimeUnit, zone: TimeZone) {
  const value = input.trim();
  const scale = unit === "seconds" ? 1000 : 1;
  if (!reverse) {
    if (!(unit === "seconds" ? /^-?\d+(\.\d{1,3})?$/ : /^-?\d+$/).test(value)) throw new Error("请输入有效的时间戳。");
    return dateText(new Date(Math.round(Number(value) * scale)), zone);
  }
  const parts = /^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2})(?:\.(\d{1,3}))?$/.exec(value);
  if (!parts) throw new Error("请输入 YYYY-MM-DD HH:mm:ss，可选填毫秒。");
  const fraction = (parts[3] ?? "").padEnd(3, "0");
  const date = new Date(parts[1] + "T" + parts[2] + "." + fraction + (zone === "utc" ? "Z" : ""));
  const expected = parts[1] + " " + parts[2] + (fraction === "000" ? "" : "." + fraction);
  // Date normalizes impossible dates and local times inside daylight-saving gaps.
  if (!Number.isFinite(date.getTime()) || dateText(date, zone) !== expected) throw new Error("此日期或时间在所选时区不存在。");
  return String(date.getTime() / scale);
}
