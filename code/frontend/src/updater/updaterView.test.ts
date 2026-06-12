import { describe, expect, it } from 'vitest';
import { describeUpdateStatus } from './updaterView';
import type { UpdateInfo } from './types';

const info: UpdateInfo = {
  current_version: '0.1.0',
  latest_version: '0.2.2',
  min_version: '0.2.0',
  notes: 'fixes',
  update_available: true,
  update_required: true,
  download_url: 'https://example/win-setup.exe',
  download_page_url: 'https://example/download',
};

describe('describeUpdateStatus', () => {
  it('idle (non-Tauri) offers no actions', () => {
    const d = describeUpdateStatus('idle', null, null);
    expect(d.showInstall).toBe(false);
    expect(d.showDownload).toBe(false);
  });

  it('available offers both install and download', () => {
    const d = describeUpdateStatus('available', info, null);
    expect(d.tone).toBe('available');
    expect(d.showInstall).toBe(true);
    expect(d.showDownload).toBe(true);
    expect(d.detail).toContain('0.2.2');
  });

  it('forced is blocking-tone and steers to update', () => {
    const d = describeUpdateStatus('forced', info, null);
    expect(d.tone).toBe('forced');
    expect(d.showInstall).toBe(true);
    expect(d.detail).toContain('no longer supported');
  });

  it('error hides in-app install but keeps manual download', () => {
    const d = describeUpdateStatus('error', null, 'manifest HTTP 500');
    expect(d.tone).toBe('error');
    expect(d.showInstall).toBe(false);
    expect(d.showDownload).toBe(true);
    expect(d.detail).toContain('manifest HTTP 500');
    expect(d.detail).toContain('manually');
  });

  it('uptodate shows current version, download still available', () => {
    const d = describeUpdateStatus('uptodate', { ...info, update_available: false }, null);
    expect(d.showInstall).toBe(false);
    expect(d.showDownload).toBe(true);
    expect(d.detail).toContain('0.1.0');
  });
});
