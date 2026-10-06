$ErrorActionPreference = 'Stop'

# Install-ChocolateyZipPackage auto-tracks the extracted files; Chocolatey
# removes the package's tools dir and the auto-generated .exe shims on
# `choco uninstall talos`. Nothing extra to clean up here.
