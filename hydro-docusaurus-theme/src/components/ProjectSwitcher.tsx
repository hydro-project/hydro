import React, {useState, useRef, useEffect, type ReactNode} from 'react';
import Link from '@docusaurus/Link';

const InfinityIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M18.178 8c5.096 0 5.096 8 0 8-5.095 0-7.133-8-12.739-8-4.585 0-4.585 8 0 8 5.606 0 7.644-8 12.74-8z" />
  </svg>
);

const ChevronDown = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 4.5L6 7.5L9 4.5" />
  </svg>
);

export type Project = {
  id: string;
  label: string;
  href: string;
  description: string;
  logo?: ReactNode;
  navbarLogo?: ReactNode;
  chevronOffset?: number;
};

const defaultProjects: Project[] = [
  {
    id: 'infinity',
    label: 'Infinity',
    href: 'https://infinity.hydro.run',
    description: 'The open-source ecosystem for agents with principled concurrency',
    logo: <><InfinityIcon /> <span>Infinity</span></>,
  },
  {
    id: 'hydro',
    label: 'Hydro',
    href: 'https://hydro.run',
    description: 'A Rust framework for correct and performant distributed systems',
    logo: <img src="/img/hydro-logo.svg" alt="Hydro" style={{height: '36px', marginLeft: '-4px'}} />,
    navbarLogo: <img src="/img/hydro-logo.svg" alt="Hydro" style={{height: '2.9rem', marginLeft: '-8px', marginTop: '-4px', marginBottom: '-4px'}} />,
    chevronOffset: -12,
  },
];

export interface ProjectSwitcherProps {
  currentProject: string;
  projects?: Project[];
}

export default function ProjectSwitcher({currentProject, projects = defaultProjects}: ProjectSwitcherProps): ReactNode {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

    const current = projects.find(p => p.id === currentProject) || projects[0];
    const sortedProjects = [current, ...projects.filter(p => p.id !== currentProject)];

  return (
    <div className="project-switcher" ref={ref}>
      <div className="project-switcher__button">
        <Link to="/" style={{display: 'flex', alignItems: 'center', gap: '6px', color: 'inherit', textDecoration: 'none'}}>
          <div className="project-switcher__logo">{current.navbarLogo || current.logo}</div>
        </Link>
        <button
          onClick={() => setOpen(!open)}
          aria-expanded={open}
          aria-label="Switch project"
          className="project-switcher__chevron"
          style={current.chevronOffset ? {marginLeft: `${current.chevronOffset}px`} : undefined}
        >
          <ChevronDown />
        </button>
      </div>
      {open && (
        <div className="project-switcher__dropdown">
          {sortedProjects.map(project => {
            const isLocal = project.id === currentProject;
            if (isLocal) {
              return (
                <Link
                  key={project.id}
                  to="/"
                  className="project-switcher__item project-switcher__item--active"
                  onClick={() => setOpen(false)}
                >
                  <div className="project-switcher__logo">{project.logo}</div>
                  <div className="project-switcher__desc">{project.description}</div>
                </Link>
              );
            }
            return (
              <a
                key={project.id}
                href={project.href}
                className="project-switcher__item"
                target="_blank"
                rel="noopener noreferrer"
                onClick={() => setOpen(false)}
              >
                <div className="project-switcher__logo">{project.logo}</div>
                <div className="project-switcher__desc">{project.description}</div>
              </a>
            );
          })}
        </div>
      )}
    </div>
  );
}
