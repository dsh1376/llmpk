import { balanced } from "./rsc.ts";

// Parse the bundle's data subset. Never evaluate code from a remote site.
export function parseModelsLiteral(bundle: string): unknown {
  const marker = bundle.indexOf("models:[{id:");
  if (marker < 0) throw new Error("no model table found in Slopalytics bundle");
  const source = balanced(bundle, marker + "models:".length);
  let i = 0;
  const space = () => {
    while (/\s/.test(source[i] ?? "x")) i++;
  };
  function string(): string {
    const quote = source[i++];
    let result = "";
    while (i < source.length) {
      const c = source[i++];
      if (c === quote) return result;
      if (quote === "`" && c === "$" && source[i] === "{")
        throw new Error("template interpolation is not data");
      if (c !== "\\") {
        result += c;
        continue;
      }
      const escaped = source[i++];
      const escapes: Record<string, string> = {
        n: "\n",
        t: "\t",
        r: "\r",
        "\\": "\\",
        "'": "'",
        '"': '"',
        "`": "`",
        $: "$",
      };
      if (!escaped || !(escaped in escapes)) throw new Error("unsupported bundle string escape");
      result += escapes[escaped];
    }
    throw new Error("unterminated bundle string");
  }
  function read(): unknown {
    space();
    const c = source[i];
    if (c && "\"'`".includes(c)) return string();
    if (c === "{" || c === "[") {
      i++;
      const array = c === "[";
      const values: unknown[] = [];
      const entries: [string, unknown][] = [];
      const close = array ? "]" : "}";
      space();
      while (source[i] !== close) {
        if (array) values.push(read());
        else {
          space();
          let key: string;
          if ("\"'`".includes(source[i] ?? " ")) key = string();
          else {
            const match = /^[A-Za-z_$][\w$]*/.exec(source.slice(i));
            if (!match) throw new Error("invalid bundle object key");
            key = match[0];
            i += key.length;
          }
          space();
          if (source[i++] !== ":") throw new Error("expected bundle key separator");
          entries.push([key, read()]);
        }
        space();
        if (source[i] === close) break;
        if (source[i++] !== ",") throw new Error("unsupported bundle expression");
        space();
      }
      i++;
      return array ? values : Object.fromEntries(entries);
    }
    const token =
      /^(?:void\s+0|!0|!1|true|false|null|-?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?)/i.exec(
        source.slice(i),
      )?.[0];
    if (!token) throw new Error("unsupported bundle expression");
    i += token.length;
    if (token === "!0" || token === "true") return true;
    if (token === "!1" || token === "false") return false;
    if (token === "null" || token.startsWith("void")) return null;
    const number = Number(token);
    if (!Number.isFinite(number)) throw new Error("invalid bundle number");
    return number;
  }
  const result = read();
  space();
  if (i !== source.length) throw new Error("trailing bundle expression");
  return result;
}
