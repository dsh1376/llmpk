export function balanced(source: string, start: number): string {
  const stack: string[] = [];
  let quote = "";
  for (let i = start; i < source.length; i++) {
    const c = source[i] ?? "";
    if (quote) {
      if (c === "\\") i++;
      else if (c === quote) quote = "";
    } else if ("\"'`".includes(c)) quote = c;
    else if (c === "[" || c === "{") stack.push(c);
    else if (c === "]" || c === "}") {
      if (stack.pop() !== (c === "]" ? "[" : "{")) throw new Error("unbalanced data");
      if (!stack.length) return source.slice(start, i + 1);
    }
  }
  throw new Error("unterminated data");
}

export function extractStream(html: string): string {
  const chunks = [...html.matchAll(/self\.__next_f\.push\(\[1,\s*("(?:[^"\\]|\\.)*")\]\)/gs)];
  const stream = chunks.map((match) => JSON.parse(match[1] ?? '""') as string).join("");
  if (!stream) throw new Error("no Next.js data stream found");
  return stream;
}

export function afterKey(source: string, key: string, opener: "[" | "{"): string {
  const match = new RegExp(`"${key}"\\s*:\\s*([\\[{])`).exec(source);
  if (!match || match[1] !== opener) throw new Error(`no ${key} data found`);
  return balanced(source, match.index + match[0].length - 1);
}

export function objectsWith(source: string, key: string): unknown[] {
  const stack: { start: number; hasMatch: boolean }[] = [];
  const spans: { start: number; body: string }[] = [];
  let quoted = false;
  for (let i = 0; i < source.length; i++) {
    const c = source[i];
    if (quoted) {
      if (c === "\\") i++;
      else if (c === '"') quoted = false;
    } else if (c === '"') quoted = true;
    else if (c === "{") stack.push({ start: i, hasMatch: false });
    else if (c === "}") {
      const span = stack.pop();
      if (!span) continue;
      const body = source.slice(span.start, i + 1);
      if (body.includes(`"${key}":`)) {
        if (!span.hasMatch) spans.push({ start: span.start, body });
        for (const parent of stack) parent.hasMatch = true;
      }
    }
  }
  return spans.sort((a, b) => a.start - b.start).map((span) => JSON.parse(span.body));
}
