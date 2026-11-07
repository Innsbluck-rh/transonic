import { createUniqueId, type JSXElement } from 'solid-js';
import Heading3 from '~/components/common/text/Heading3';

type SettingSubSectionProps = {
  title: string;
  children: JSXElement;
};

function SettingSubSection(props: SettingSubSectionProps) {
  const headingId = createUniqueId();
  return (
    <div role='group' aria-labelledby={headingId} class='flex flex-col'>
      <Heading3 id={headingId} class='mb-4'>
        {props.title}
      </Heading3>
      <div class='flex flex-col gap-3'>{props.children}</div>
    </div>
  );
}

export default SettingSubSection;
