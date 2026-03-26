import { Component, Show } from 'solid-js';
import { sessionStore } from '~/stores/session/SessionStore';
import Title from '../common/Title';
import Profiles from './Profiles';

interface HeaderProps {
  title?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  return (
    <div class='flex flex-row items-center border-b border-zinc-400 px-3 py-2'>
      <Title>{props.title ?? 'Transonic'}</Title>
      <div class='flex-1' />
      <Show when={props.shouldShowProfiles}>
        <Profiles profiles={sessionStore.profiles} />
      </Show>
    </div>
  );
};

export default Header;
