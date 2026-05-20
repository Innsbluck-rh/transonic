import type { JSXElement } from 'solid-js';
import Heading2 from '~/components/common/Heading2';

type SettingSectionProps = {
  title: string;
  children: JSXElement;
  headerContent?: JSXElement;
};

function SettingSection(props: SettingSectionProps) {
  return (
    <section class='flex min-w-0 flex-col gap-1'>
      <Heading2>{props.title}</Heading2>
      {props.headerContent}
      <div class='bg-primary-plane border-secondary-border flex flex-col gap-3 rounded-md border p-4'>{props.children}</div>
    </section>
  );
}

export default SettingSection;
