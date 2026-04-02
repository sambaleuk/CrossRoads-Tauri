import { check } from '@tauri-apps/plugin-updater';
import { ask } from '@tauri-apps/plugin-dialog';

/**
 * Check for updates on app startup. If an update is available,
 * prompt the user and install if they accept.
 */
export async function checkForUpdates(silent = false): Promise<void> {
  try {
    const update = await check();
    if (!update) {
      if (!silent) console.log('No update available');
      return;
    }

    console.log(`Update available: ${update.version} (current: ${update.currentVersion})`);

    const yes = await ask(
      `A new version of XRoads is available!\n\nCurrent: ${update.currentVersion}\nNew: ${update.version}\n\nWould you like to update now?`,
      { title: 'XRoads Update', kind: 'info' }
    );

    if (yes) {
      console.log('Downloading and installing update...');
      await update.downloadAndInstall();
      // The app will restart automatically after install
    }
  } catch (e) {
    if (!silent) console.warn('Update check failed:', e);
  }
}
