import Heading1 from '~/components/common/Heading1';
import AlbumArtCacheSettings from '~/components/settings/AlbumArtCacheSettings';
import AppearanceSettings from '~/components/settings/AppearanceSettings';
import ConnectSettings from '~/components/settings/ConnectSettings';
import PlaybackSettings from '~/components/settings/PlaybackSettings';
import ServerSettings from '~/components/settings/ServerSettings';

function SettingsRoute() {
  return (
    <div class='home-surface-root gap-4 p-4 lg:p-5'>
      <Heading1>Settings</Heading1>
      <AppearanceSettings />
      <PlaybackSettings />
      <ServerSettings />
      <AlbumArtCacheSettings />
      <ConnectSettings />
    </div>
  );
}

export default SettingsRoute;
