/** Resolve the public installation target without changing Git remotes. */
export function validateReleaseTarget({ manifest, installSh, installPs1, origin, mirrorPublisherConfigured = false }) {
  const publish = manifest.match(/^\[publish\]\s*\n([\s\S]*)/m)?.[1]?.split(/^\[/m)[0];
  const mirror = publish?.match(/^to\s*=\s*"https:\/\/github\.com\/([^"\s]+?)(?:\.git)?"/m)?.[1];
  const branch = publish?.match(/^branch\s*=\s*"([^"\s]+)"/m)?.[1];
  if (branch && branch !== "main") throw new Error("Public release exports must target main.");
  const unix = installSh.match(/^REPO="([^"]+)"/m)?.[1];
  const windows = installPs1.match(/^\$Repo = '([^']+)'/m)?.[1];
  if (!mirror || !unix || !windows) throw new Error("Cannot resolve public release destinations from domain.toml and installers.");
  if (unix !== windows || mirror !== unix) {
    throw new Error(`Release destinations disagree: publish=${mirror}, install.sh=${unix}, install.ps1=${windows}. Resolve the approved public mirror before releasing.`);
  }
  const source = origin.match(/github\.com[:/]([^\s]+?)(?:\.git)?$/)?.[1];
  if (!source) throw new Error("Cannot resolve source GitHub repository from origin.");
  if (source !== mirror && !mirrorPublisherConfigured) {
    throw new Error(`Source ${source} differs from public mirror ${mirror}. The current CI publishes assets only to the source; wire and verify the gated public export and asset publication before releasing.`);
  }
  return source;
}
