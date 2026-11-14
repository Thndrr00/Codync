import { DeviceMobile, UsersThree, ChatsCircle, UserPlus } from "@phosphor-icons/react/ssr";
import Card from "./card";
import Reveal from "./reveal";
import SectionHeader from "./section-header";

const points = [
  {
    icon: UserPlus,
    title: "Everyone on the same bots",
    body: "Whoever you trust pairs their own phone with the computer once. They see the same bots, rooms and files you do, and answer as themselves.",
  },
  {
    icon: ChatsCircle,
    title: "Bots ask the room",
    body: "A bot meeting a choice asks the people in a group chat with a question card. The first two answers come back to its turn, so it keeps going instead of stopping.",
  },
  {
    icon: UsersThree,
    title: "Bots work with each other",
    body: "One bot can hand a task to another and get the reply back as its own turn, or ask one on another of your computers. Every exchange is a row you can open.",
  },
];

// The people and the bot in the room, drawn like a group chat (docs/features/groups-and-replies.md).
const room = [
  { from: "Maya", hue: 255, text: "Ship the pricing fix today. Reviewer, check it first." },
  { from: "Reviewer", bot: true, hue: 150, text: "Two tests fail on the rounding path. Do we hold the release?" },
  { from: "Sam", hue: 30, text: "Hold it. Maya, is the invoice total the same bug?" },
];

// Several people and a few bots, sharing one computer's rooms.
export default function Multiplayer() {
  return (
    <section className="px-4 py-24 sm:px-6 md:py-32">
      <div className="mx-auto max-w-6xl">
        <SectionHeader label="Multiplayer mode" title="Your team, in the room with the bots.">
          Codync is not a single-player remote. Invite the people you work with to the same computer and the same bots.
        </SectionHeader>

        <div className="mt-16 grid grid-cols-1 items-center gap-12 md:grid-cols-[1fr_1fr] md:gap-20">
          <Reveal>
            <ul className="flex flex-col gap-8">
              {points.map(({ icon: Icon, title, body }) => (
                <li key={title} className="flex gap-4">
                  <Icon size={24} className="mt-0.5 shrink-0 text-neutral-50" />
                  <div>
                    <h3 className="text-lg font-semibold text-neutral-50">{title}</h3>
                    <p className="mt-1.5 leading-relaxed text-neutral-400">{body}</p>
                  </div>
                </li>
              ))}
            </ul>
          </Reveal>
          <Reveal delay={0.05}>
            <Card
              title="Release room"
              body="Maya, Sam and a reviewer, in one chat."
              wellClassName="bg-neutral-950 p-4"
            >
              <ul className="flex flex-col gap-3">
                {room.map(({ from, bot, hue, text }) => (
                  <li key={text} className="flex gap-3">
                    <span
                      aria-hidden
                      className="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-semibold text-neutral-950"
                      style={{ backgroundColor: `hsl(${hue} 62% 68%)` }}
                    >
                      {from.slice(0, 1)}
                    </span>
                    <span className="rounded-2xl bg-neutral-900 px-4 py-3">
                      <span className="block text-sm font-medium text-neutral-50">
                        {from}
                        {bot ? <span className="font-normal text-neutral-500"> · bot</span> : null}
                      </span>
                      <span className="mt-1 block text-sm leading-relaxed text-neutral-300">{text}</span>
                    </span>
                  </li>
                ))}
                <li className="flex items-center gap-3 rounded-2xl border border-dashed border-neutral-700 px-4 py-3 text-sm text-neutral-400">
                  <DeviceMobile size={18} className="shrink-0" />
                  Your phone, Maya&apos;s phone and Sam&apos;s laptop, paired with one computer.
                </li>
              </ul>
            </Card>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
