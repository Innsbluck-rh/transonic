import { useLocation, useNavigate } from '@solidjs/router';
import { Component, Show } from 'solid-js';
import { sessionStore } from '~/stores/SessionStore';
import Title from '../common/Title';
import ProfilesNew from './Profiles';

interface HeaderProps {
  title?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <div class='flex flex-row items-center h-14 border-b border-zinc-400 px-2.5'>
      <Title class='cursor-pointer' onClick={() => navigate('/home')}>
        {props.title ?? 'Transonic'}
      </Title>
      <div class='flex-1' />
      <p class='text-xs opacity-25 mr-3'>{location.pathname}</p>
      <Show when={props.shouldShowProfiles}>
        <ProfilesNew profiles={sessionStore.profiles} />
      </Show>
    </div>
  );
};

export default Header;
