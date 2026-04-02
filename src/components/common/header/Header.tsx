import { useNavigate } from '@solidjs/router';
import { Component, Show } from 'solid-js';
import { sessionStore } from '~/stores/SessionStore';
import Title from '../Title';
import ProfilesNew from './ServerProfile';

interface HeaderProps {
  title?: string;
  titleHref?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  const navigate = useNavigate();

  return (
    <div class='bg-primary-plane border-primary-border flex h-12 flex-row items-center border-b px-2.5'>
      <Show when={props.titleHref} fallback={<Title>{props.title ?? 'Transonic'}</Title>}>
        <Title class='cursor-pointer' onClick={() => navigate(props.titleHref!)}>
          {props.title ?? 'Transonic'}
        </Title>
      </Show>
      <div class='flex-1' />
      {/* <input class='text-xs mr-2' placeholder='search...' /> */}
      <Show when={props.shouldShowProfiles}>
        <ProfilesNew profiles={sessionStore.profiles} />
      </Show>
    </div>
  );
};

export default Header;
