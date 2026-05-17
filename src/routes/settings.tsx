import Heading1 from '~/components/common/Heading1';
import AlbumArtCacheSettings from '~/components/settings/AlbumArtCacheSettings';
import AppearanceSettings from '~/components/settings/AppearanceSettings';
import ConnectSettings from '~/components/settings/ConnectSettings';
import PlaybackSettings from '~/components/settings/PlaybackSettings';
import ServerSettings from '~/components/settings/ServerSettings';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SettingsRoute() {
  return (
    <div class='home-surface-root gap-4 p-4 lg:p-5'>
      <div
        class='flex flex-col gap-4'
        style={{
          'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
          'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        }}
      >
        <Heading1>Settings</Heading1>
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
