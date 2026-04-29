import { readFileSync, writeFileSync } from "node:fs";

const indexPath = "web-build/index.html";

const html = readFileSync(indexPath, "utf-8")
  .replaceAll('href="/', 'href="./')
  .replaceAll('src="/', 'src="./')
  .replaceAll('import("/', 'import("./');

writeFileSync(indexPath, html);
