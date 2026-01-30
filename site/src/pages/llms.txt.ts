import type { APIRoute } from "astro";
import { getCollection } from "astro:content";
import { examples } from "../data/examples";

export const GET: APIRoute = async () => {
  // Get all docs from the content collection
  const docs = await getCollection("docs");

  // Sort by order field
  docs.sort((a, b) => (a.data.order ?? 99) - (b.data.order ?? 99));

  const content = `# Hone Documentation

> Hone is a CLI testing tool with a simple DSL for writing readable, maintainable tests.

## Documentation

${docs
  .map(
    (doc) => `## ${doc.data.title}
URL: https://hone.safia.dev/docs/${doc.slug}

${doc.body}
`
  )
  .join("\n---\n\n")}

---

## Examples

Real-world examples of Hone test files. Copy and adapt these for your own CLI tests.

${examples
  .map(
    (example) => `### ${example.title}

${example.description}

Filename: ${example.filename}

\`\`\`hone
${example.code}
\`\`\`
`
  )
  .join("\n---\n\n")}
`;

  return new Response(content, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
};
