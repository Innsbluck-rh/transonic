import type { JSXElement } from 'solid-js';
import Heading2 from '~/components/common/Heading2';

type SettingSectionProps = {
  title: string;
  children: JSXElement;
  headerContent?: JSXElement;
};

function SettingSection(props: SettingSectionProps) {
  return (
    <section class='bg-primary-plane border-primary-border flex min-w-0 flex-col gap-3 rounded-lg border p-4'>
      <Heading2>{props.title}</Heading2>
      {props.headerContent}
      {props.children}
    </section>
  );
}

export default SettingSection;
