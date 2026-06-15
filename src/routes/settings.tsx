import RouteHeader from '~/components/common/text/RouteHeader';
import AlbumArtCacheSettings from '~/components/settings/AlbumArtCacheSettings';
import AppearanceSettings from '~/components/settings/AppearanceSettings';
import ConnectSettings from '~/components/settings/ConnectSettings';
import PlaybackSettings from '~/components/settings/PlaybackSettings';
import ServerSettings from '~/components/settings/ServerSettings';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SettingsRoute() {
  return (
    <div
      class='home-surface-root relative p-0'
      style={{
        'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
      }}
    >
      <RouteHeader title='Settings' />
      <div class='flex flex-1 flex-col gap-4 px-4 py-4 lg:p-5'>
        <ServerSettings />
        <AppearanceSettings />
        <PlaybackSettings />
        <AlbumArtCacheSettings />
        <ConnectSettings />
      </div>
    </div>
  );
}

export default SettingsRoute;
