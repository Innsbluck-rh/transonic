import RouteHeader from '~/components/common/RouteHeader';
import AlbumArtCacheSettings from '~/components/settings/AlbumArtCacheSettings';
import AppearanceSettings from '~/components/settings/AppearanceSettings';
import ConnectSettings from '~/components/settings/ConnectSettings';
import PlaybackSettings from '~/components/settings/PlaybackSettings';
import ServerSettings from '~/components/settings/ServerSettings';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SettingsRoute() {
  return (
    <div
      class='home-surface-root p-0'
      style={{
        'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
      }}
    >
      <RouteHeader title='Settings' />
      <div class='flex flex-1 flex-col gap-4 p-3 lg:p-5'>
        <AppearanceSettings />
        <PlaybackSettings />
        <ServerSettings />
        <AlbumArtCacheSettings />
        <ConnectSettings />
      </div>
    </div>
  );
}

export default SettingsRoute;
