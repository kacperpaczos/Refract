import configparser
from pathlib import Path


class UserConf:
    """
    class for read/write key-values in $HOME
    """
    KEY = 'SETTING'
    __app_name__ = "gnome-layout-switcher"

    def __init__(self, app_name: str = ''):
        """
        initialize object

        Args:
            app_name (str): name of application
        """

        if not app_name:
            app_name = self.__app_name__
        self.inifile = f"{Path.home()}/.config/{app_name}.conf"
        # self.inifile.parent.mkdir(parents=True, exist_ok=True)
        self.config = None  # can use only after "with"
        self.modified = False

    def __enter__(self) -> object:
        """
        enter in "with" ...
        """

        self.config = configparser.ConfigParser()
        self.config.read(self.inifile)
        self.modified = False
        # create default session "KEY" if not exists
        try:
            self.config[self.KEY]
        except KeyError:
            self.config[self.KEY] = {}

        return self

    def __exit__(self, etype, evalue, traceback) -> None:
        self.save()

    def save(self):
        if self.modified:
            with open(self.inifile, 'wt', encoding="utf-8") as configfile:
                self.config.write(configfile)

    def read(self, key: str, default=None):
        """
        read a key in .conf file

        Args:
            key (str): key format: section.key , section default is self.KEY
            default (optional): default return if key not exists

        Returns:
            [string, bool, optional]: value for key
        """

        section, *keys = key.split('.', 2)
        if not keys:
            key = section
            section = self.KEY
        else:
            section = section.upper()
            key = keys[0]

        try:
            return self.config.get(section, key)
        except (KeyError, configparser.NoOptionError, configparser.NoSectionError):
            return default or None

    def write(self, fields: dict, section: str = ''):
        """
        set values in .conf

        Args:
            fields (dict): datas
            section (str, optional): section name. Default is self.KEY
        """

        if not section:
            section = self.KEY
        section = section.upper()
        section_object = None
        try:
            section_object = self.config[section]
        except KeyError:
            self.config[section] = {}

        for key, local_val in fields.items():
            section_object[key] = str(local_val)
        self.modified = True


if __name__ == '__main__':
    """
    how to
    """
    conf = UserConf()
    with conf:
        # or: with UserConf() as conf:
        conf.write({'layout': 'manjaro'})  # in default section
        conf.write({'first': 'True'}, 'TEST')
        conf.write({'x': 55, 'y': 199, 'z': 1}, 'SIZE')

    with UserConf() as user:
        value = user.read('setting.layout', '?')
        print("value default section , key layout:", value)
        value = user.read('layout', '?')
        print("value default section , key layout:", value)
