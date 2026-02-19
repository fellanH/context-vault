export function PrivacyPage() {
  return (
    <main>
      <section className="mx-auto w-full max-w-3xl px-6 py-16 sm:py-20">
        <div className="space-y-3 mb-10">
          <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
            Privacy Policy
          </h1>
          <p className="text-sm text-muted-foreground">
            Last updated: February 18, 2026
          </p>
        </div>

        <article className="prose prose-neutral dark:prose-invert max-w-none space-y-8">
          <section>
            <h2 className="text-xl font-semibold mb-3">
              What this extension stores
            </h2>
            <ul className="list-disc pl-5 space-y-1.5 text-muted-foreground">
              <li>Your configured Context Vault server URL</li>
              <li>
                Your API key used to authenticate to your own Context Vault
                server
              </li>
              <li>
                Temporary rate-limit metadata from server response headers
              </li>
            </ul>
          </section>

          <section>
            <h2 className="text-xl font-semibold mb-3">How data is used</h2>
            <ul className="list-disc pl-5 space-y-1.5 text-muted-foreground">
              <li>
                Selected text is sent only to the server URL you configure.
              </li>
              <li>
                Search queries are sent only to your configured server.
              </li>
              <li>
                No analytics, ad tracking, or third-party telemetry is collected
                by this extension.
              </li>
            </ul>
          </section>

          <section>
            <h2 className="text-xl font-semibold mb-3">
              What is not collected
            </h2>
            <ul className="list-disc pl-5 space-y-1.5 text-muted-foreground">
              <li>
                No browsing history collection beyond pages where the content
                script runs.
              </li>
              <li>No sale or sharing of personal data.</li>
              <li>
                No transfer of data to external processors outside your
                configured Context Vault server.
              </li>
            </ul>
          </section>

          <section>
            <h2 className="text-xl font-semibold mb-3">
              How to delete your data
            </h2>
            <p className="text-muted-foreground">
              Remove stored extension settings from Chrome by opening extension
              settings and clearing values, or uninstall the extension to remove
              locally stored data. To delete vault entries, use your Context
              Vault app or API.
            </p>
          </section>

          <section>
            <h2 className="text-xl font-semibold mb-3">Contact</h2>
            <p className="text-muted-foreground">
              Project repository:{" "}
              <a
                href="https://github.com/fellanH/context-mcp"
                target="_blank"
                rel="noreferrer"
                className="text-foreground underline underline-offset-4 hover:text-primary"
              >
                github.com/fellanH/context-mcp
              </a>
            </p>
          </section>
        </article>
      </section>
    </main>
  );
}
