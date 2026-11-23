// One-shot critique from a different model family (OpenAI Responses API). Used for script and round reviews.
// usage: OPENAI_API_KEY=... bun critic.ts <prompt.md> <out.md> [--model gpt-6.1-sol] [file.md|image.png ...]
import { readFileSync, writeFileSync } from "node:fs"

const args = process.argv.slice(2)
const mi = args.indexOf("--model")
const model = mi >= 0 ? args.splice(mi, 2)[1] : "gpt-6.1-sol"
const [promptPath, out, ...files] = args
const content: unknown[] = [{ type: "input_text", text: readFileSync(promptPath, "utf8") }]
for (const f of files) {
  if (/\.(png|jpe?g|webp)$/i.test(f)) {
    const mime = f.endsWith(".png") ? "image/png" : f.endsWith(".webp") ? "image/webp" : "image/jpeg"
    content.push({ type: "input_text", text: `Image: ${f.split("/").pop()}` })
    content.push({ type: "input_image", image_url: `data:${mime};base64,${readFileSync(f).toString("base64")}` })
  } else content.push({ type: "input_text", text: `--- ${f.split("/").pop()} ---\n${readFileSync(f, "utf8")}` })
}
const res = await fetch("https://api.openai.com/v1/responses", {
  method: "POST",
  headers: { "content-type": "application/json", authorization: `Bearer ${process.env.OPENAI_API_KEY}` },
  body: JSON.stringify({ model, reasoning: { effort: "high" }, input: [{ role: "user", content }] }),
})
const json: any = await res.json()
if (!res.ok) {
  console.error(JSON.stringify(json).slice(0, 2000))
  process.exit(1)
}
const text = json.output_text ?? json.output?.flatMap((o: any) => o.content ?? []).filter((c: any) => c.type === "output_text").map((c: any) => c.text).join("\n")
writeFileSync(out, `<!-- ${model}, ${new Date().toISOString()} -->\n\n${text}\n`)
console.log(`${model}: ${json.usage?.input_tokens} in / ${json.usage?.output_tokens} out -> ${out}`)
