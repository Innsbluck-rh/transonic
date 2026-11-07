import type { JSXElement } from 'solid-js';
import Heading2 from '~/components/common/text/Heading2';

type SettingSectionProps = {
  title: string;
  children: JSXElement;
  sectionId?: string;
};

function SettingSection(props: SettingSectionProps) {
  return (
    <section id={props.sectionId} class='flex w-full flex-col'>
      <div class='bg-primary-plane border-secondary-border flex w-full flex-col gap-5 overflow-hidden rounded-md border p-4'>
        <div class='flex flex-col opacity-50'>
          <Heading2 class='-mb-2'>{props.title}</Heading2>
          <div class='bg-secondary-text/75 h-px w-full' />
        </div>
        <div class='flex w-full flex-col gap-5 px-1 py-1'>{props.children}</div>
      </div>
    </section>
  );
}

export default SettingSection;
